// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:local_notifier/local_notifier.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:tray_manager/tray_manager.dart' as tray;
import 'package:url_launcher/url_launcher.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:window_manager/window_manager.dart';

final appVersionProvider = FutureProvider<String>((ref) async {
  final info = await PackageInfo.fromPlatform();
  return info.buildNumber.isEmpty ? info.version : '${info.version}+${info.buildNumber}';
});

/// Window close behaviour, tray icon, desktop notifications and the update check.
class DesktopIntegration extends ConsumerStatefulWidget {
  const DesktopIntegration({super.key, required this.navigatorKey, required this.child});

  final GlobalKey<NavigatorState> navigatorKey;
  final Widget child;

  @override
  ConsumerState<DesktopIntegration> createState() => _DesktopIntegrationState();
}

class _DesktopIntegrationState extends ConsumerState<DesktopIntegration> with WindowListener {
  tray.TrayIcon? _trayIcon;
  var _updateChecked = false;
  var _exiting = false;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    windowManager.setPreventClose(true);
    ref.listenManual(runtimeProvider, _onRuntime);
    ref.listenManual(settingsValueProvider, (previous, next) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) {
          return;
        }
        _syncTray(next);
        if (next != null) {
          _checkUpdates(next);
        }
      });
    }, fireImmediately: true);
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    _disposeTray();
    super.dispose();
  }

  BuildContext? get _dialogContext => widget.navigatorKey.currentContext;

  @override
  void onWindowClose() async {
    final settings = ref.read(settingsValueProvider);
    if (settings?.general.closeBehavior == CloseBehavior.minimizeToTray) {
      await windowManager.hide();
      return;
    }
    await _exit(ask: settings?.general.closeBehavior != CloseBehavior.stopAndExit);
  }

  Future<void> _exit({required bool ask}) async {
    if (_exiting) {
      return;
    }
    if (ask && anyServerRunning()) {
      final context = _dialogContext;
      if (context != null && context.mounted) {
        final l = context.l10n;
        final leave = await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(l.exitTitle),
            content: Text(l.exitMessage),
            actions: [
              TextButton(onPressed: () => Navigator.pop(context, false), child: Text(l.cancel)),
              FilledButton(onPressed: () => Navigator.pop(context, true), child: Text(l.exitConfirm)),
            ],
          ),
        );
        if (leave != true) {
          return;
        }
      }
    }
    _exiting = true;
    await shutdownAll();
    _disposeTray();
    await windowManager.setPreventClose(false);
    await windowManager.destroy();
  }

  void _syncTray(LauncherSettings? settings) {
    final wanted = settings?.general.closeBehavior == CloseBehavior.minimizeToTray;
    if (!wanted) {
      _disposeTray();
      return;
    }
    if (_trayIcon != null) {
      return;
    }
    final l = context.l10n;
    final icon = tray.TrayIcon.create();
    final menu = tray.Menu.create();
    if (icon == null || menu == null) {
      return;
    }
    tray.MenuItem? item(String label, VoidCallback onClick) {
      final entry = tray.MenuItem.createWithLabelAndType(label, tray.MenuItemType.normal);
      entry?.addListener((event) {
        if (event is tray.MenuItemClickedEvent) {
          onClick();
        }
      });
      return entry;
    }

    for (final entry in [
      item(l.trayShow, _showWindow),
      item(l.trayStopAll, () => shutdownAll()),
    ]) {
      if (entry != null) {
        menu.addItem(entry);
      }
    }
    menu.addSeparator();
    final quit = item(l.trayQuit, () => _exit(ask: false));
    if (quit != null) {
      menu.addItem(quit);
    }
    try {
      icon.icon = tray.ImageAsset.fromAsset(Theme.of(context).platform == TargetPlatform.windows ? 'assets/tray/tray_icon.ico' : 'assets/tray/tray_icon.png');
    } catch (_) {
      // A missing icon must not prevent the app from running.
    }
    icon.setTooltip('VoxelPanel');
    icon.setContextMenu(menu);
    icon.addListener((event) {
      if (event is tray.TrayIconClickedEvent || event is tray.TrayIconDoubleClickedEvent) {
        _showWindow();
      }
    });
    icon.setVisible(true);
    _trayIcon = icon;
  }

  void _disposeTray() {
    _trayIcon?.setVisible(false);
    _trayIcon?.dispose();
    _trayIcon = null;
  }

  Future<void> _showWindow() async {
    await windowManager.show();
    await windowManager.focus();
  }

  void _notify(String title, String body) {
    unawaited(LocalNotification(title: title, body: body).show().catchError((_) {}));
  }

  void _onRuntime(RuntimeState? previous, RuntimeState next) {
    final settings = ref.read(settingsValueProvider)?.notifications;
    if (settings == null || previous == null) {
      return;
    }
    final l = context.l10n;
    final names = {for (final server in ref.read(serverListProvider).value ?? const <ServerSummary>[]) server.id: server.name};
    for (final entry in next.servers.entries) {
      final before = previous.servers[entry.key];
      final now = entry.value;
      final name = names[entry.key] ?? entry.key;
      if (settings.crash && now.crashed && before?.crashed != true && now.status == ServerStatus.stopped) {
        _notify(l.notifyCrashTitle(name), l.notifyCrashBody(now.lastExitCode ?? -1));
      }
      if (settings.ready && now.status == ServerStatus.running && before?.status == ServerStatus.starting) {
        _notify(l.notifyReadyTitle(name), l.notifyReadyBody);
      }
      if (settings.playerJoin) {
        for (final player in now.players.where((player) => !(before?.players.contains(player) ?? false))) {
          _notify(l.notifyPlayerTitle(name), l.notifyPlayerBody(player));
        }
      }
    }
  }

  Future<void> _checkUpdates(LauncherSettings settings) async {
    if (_updateChecked || !settings.general.checkUpdates) {
      return;
    }
    _updateChecked = true;
    try {
      final version = await ref.read(appVersionProvider.future);
      final update = await checkForUpdate(currentVersion: version);
      final context = _dialogContext;
      if (update == null || context == null || !context.mounted) {
        return;
      }
      final l = context.l10n;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          duration: const Duration(seconds: 12),
          content: Text(l.updateAvailable(update.version)),
          action: SnackBarAction(label: l.updateOpen, onPressed: () => launchUrl(Uri.parse(update.url))),
        ),
      );
    } catch (_) {
      // Offline or rate limited: the check is best effort.
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
