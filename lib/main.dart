// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:ui' show AppExitResponse;

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/home_screen.dart';
import 'package:voxel_panel/screens/settings/launcher_settings_screen.dart';
import 'package:voxel_panel/src/desktop_integration.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/frb_generated.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/window_title_bar.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  const options = WindowOptions(titleBarStyle: TitleBarStyle.hidden, title: 'VoxelPanel', minimumSize: Size(720, 520));
  await windowManager.waitUntilReadyToShow(options, () async {
    await windowManager.show();
    await windowManager.focus();
  });
  await RustLib.init();
  await initNotifications();
  runApp(const ProviderScope(child: VoxelApp()));
}

class VoxelApp extends ConsumerStatefulWidget {
  const VoxelApp({super.key});

  @override
  ConsumerState<VoxelApp> createState() => _VoxelAppState();
}

class _VoxelAppState extends ConsumerState<VoxelApp> with WidgetsBindingObserver {
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

  /// Exits not triggered by the window (logout, system shutdown) still stop the servers.
  @override
  Future<AppExitResponse> didRequestAppExit() async {
    await shutdownAll();
    return AppExitResponse.exit;
  }

  void _openSettings() {
    final navigator = _navigatorKey.currentState;
    if (navigator == null) {
      return;
    }
    var open = false;
    navigator.popUntil((route) {
      open = open || route.settings.name == LauncherSettingsScreen.routeName;
      return true;
    });
    if (!open) {
      navigator.push(MaterialPageRoute(settings: const RouteSettings(name: LauncherSettingsScreen.routeName), builder: (_) => const LauncherSettingsScreen()));
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(settingsValueProvider);
    final appearance = settings?.appearance;
    final accent = appearance == null ? defaultAccent : Color(appearance.accentColor);
    final density = appearance?.compact == true ? VisualDensity.compact : VisualDensity.standard;
    return MaterialApp(
      navigatorKey: _navigatorKey,
      title: 'VoxelPanel',
      debugShowCheckedModeBanner: false,
      theme: voxelTheme(brightness: Brightness.light, accent: accent, density: density),
      darkTheme: voxelTheme(brightness: Brightness.dark, accent: accent, density: density),
      themeMode: appearance == null ? ThemeMode.dark : themeModeOf(appearance.theme),
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      locale: settings == null ? null : localeOf(settings.general.language),
      builder: (context, child) {
        final media = MediaQuery.of(context);
        return MediaQuery(
          data: media.copyWith(textScaler: TextScaler.linear(appearance?.textScale ?? 1.0)),
          child: DesktopIntegration(
            navigatorKey: _navigatorKey,
            child: _WindowShell(onSettings: _openSettings, child: child ?? const SizedBox.shrink()),
          ),
        );
      },
      home: const HomeScreen(),
    );
  }
}

/// Title bar plus app content. It sits above the Navigator, so it needs its own Overlay
/// for the tooltips of the title bar buttons.
class _WindowShell extends StatefulWidget {
  const _WindowShell({required this.onSettings, required this.child});

  final VoidCallback onSettings;
  final Widget child;

  @override
  State<_WindowShell> createState() => _WindowShellState();
}

class _WindowShellState extends State<_WindowShell> {
  late final OverlayEntry _entry = OverlayEntry(
    builder: (context) => Column(
      children: [
        WindowTitleBar(onSettings: widget.onSettings),
        Expanded(child: widget.child),
      ],
    ),
  );

  @override
  void didUpdateWidget(_WindowShell oldWidget) {
    super.didUpdateWidget(oldWidget);
    _entry.markNeedsBuild();
  }

  @override
  void dispose() {
    _entry.remove();
    _entry.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Overlay(initialEntries: [_entry]);
}
