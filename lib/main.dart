// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:ui' show AppExitResponse;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/home_screen.dart';
import 'package:voxel_panel/screens/launcher_settings_screen.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/frb_generated.dart';
import 'package:voxel_panel/widgets/window_title_bar.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  const options = WindowOptions(titleBarStyle: TitleBarStyle.hidden, title: 'VoxelPanel');
  await windowManager.waitUntilReadyToShow(options, () async {
    await windowManager.show();
    await windowManager.focus();
  });
  await RustLib.init();
  runApp(const ProviderScope(child: VoxelApp()));
}

class VoxelApp extends StatefulWidget {
  const VoxelApp({super.key});

  @override
  State<VoxelApp> createState() => _VoxelAppState();
}

class _VoxelAppState extends State<VoxelApp> with WidgetsBindingObserver {
  final _navigatorKey = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  Future<AppExitResponse> didRequestAppExit() async {
    if (!anyServerRunning()) {
      await shutdownAll();
      return AppExitResponse.exit;
    }
    final context = _navigatorKey.currentContext;
    if (context == null || !context.mounted) {
      await shutdownAll();
      return AppExitResponse.exit;
    }
    final leave = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Chiudere VoxelPanel?'),
        content: const Text('I server avviati da VoxelPanel verranno fermati.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Annulla')),
          FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Ferma e chiudi')),
        ],
      ),
    );
    if (leave == true) {
      await shutdownAll();
      return AppExitResponse.exit;
    }
    return AppExitResponse.cancel;
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      navigatorKey: _navigatorKey,
      title: 'VoxelPanel',
      theme: voxelTheme(),
      builder: (context, child) {
        return _LauncherShell(child: child ?? const SizedBox.shrink());
      },
      home: const HomeScreen(),
    );
  }
}

class _LauncherShell extends StatefulWidget {
  const _LauncherShell({required this.child});

  final Widget child;

  @override
  State<_LauncherShell> createState() => _LauncherShellState();
}

class _LauncherShellState extends State<_LauncherShell> {
  final _overlayKey = GlobalKey<OverlayState>();
  OverlayEntry? _settings;

  void _toggleSettings() {
    final overlay = _overlayKey.currentState;
    if (overlay == null) {
      return;
    }
    if (_settings != null) {
      _settings!.remove();
      _settings = null;
      return;
    }
    _settings = OverlayEntry(
      builder: (context) => Positioned.fill(
        top: 40,
        child: Material(
          color: panelBackground,
          child: LauncherSettingsScreen(onClose: _toggleSettings),
        ),
      ),
    );
    overlay.insert(_settings!);
  }

  @override
  void dispose() {
    _settings?.remove();
    _settings = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Overlay(
      key: _overlayKey,
      initialEntries: [
        OverlayEntry(
          builder: (context) => Column(
            children: [
              WindowTitleBar(onSettings: _toggleSettings),
              Expanded(child: widget.child),
            ],
          ),
        ),
      ],
    );
  }
}
