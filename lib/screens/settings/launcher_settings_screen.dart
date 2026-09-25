// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/settings/sections_general.dart';
import 'package:voxel_panel/screens/settings/sections_system.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

/// Saves [next] and reports validation errors from Rust.
Future<void> saveSettings(BuildContext context, WidgetRef ref, LauncherSettings next) async {
  try {
    await ref.read(launcherSettingsProvider.notifier).save(next);
  } catch (error) {
    if (context.mounted) {
      showError(context, error);
    }
  }
}

class LauncherSettingsScreen extends ConsumerStatefulWidget {
  const LauncherSettingsScreen({super.key});

  static const routeName = '/settings';

  @override
  ConsumerState<LauncherSettingsScreen> createState() => _LauncherSettingsScreenState();
}

class _LauncherSettingsScreenState extends ConsumerState<LauncherSettingsScreen> {
  var _section = 0;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final settings = ref.watch(launcherSettingsProvider);
    final sections = <(SidebarEntry, Widget Function(LauncherSettings))>[
      (SidebarEntry(label: l.settingsGeneral, icon: Icons.tune), (s) => GeneralSection(settings: s)),
      (SidebarEntry(label: l.settingsAppearance, icon: Icons.palette_outlined), (s) => AppearanceSection(settings: s)),
      (SidebarEntry(label: l.settingsPaths, icon: Icons.folder_outlined), (s) => PathsSection(settings: s)),
      (SidebarEntry(label: l.settingsJava, icon: Icons.coffee_outlined), (s) => JavaSection(settings: s)),
      (SidebarEntry(label: l.settingsDefaults, icon: Icons.add_box_outlined), (s) => DefaultsSection(settings: s)),
      (SidebarEntry(label: l.settingsConsole, icon: Icons.terminal), (s) => ConsoleSection(settings: s)),
      (SidebarEntry(label: l.settingsBackup, icon: Icons.inventory_2_outlined), (s) => BackupSection(settings: s)),
      (SidebarEntry(label: l.settingsNetwork, icon: Icons.lan_outlined), (s) => NetworkSection(settings: s)),
      (SidebarEntry(label: l.settingsNotifications, icon: Icons.notifications_outlined), (s) => NotificationsSection(settings: s)),
      (SidebarEntry(label: l.settingsAdvanced, icon: Icons.build_outlined), (s) => AdvancedSection(settings: s)),
      (SidebarEntry(label: l.settingsAbout, icon: Icons.info_outline), (s) => const AboutSection()),
    ];
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            entries: [for (final section in sections) section.$1],
            selected: _section,
            onSelected: (index) => setState(() => _section = index),
            onBack: () => Navigator.pop(context),
            backLabel: l.back,
          ),
          Expanded(
            child: settings.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (error, stack) => ErrorState(error: error, onRetry: () => ref.invalidate(launcherSettingsProvider)),
              data: (value) => ListView(
                padding: const EdgeInsets.fromLTRB(28, 24, 28, 28),
                children: [
                  Text(sections[_section].$1.label, style: const TextStyle(fontSize: 28, fontWeight: FontWeight.w700)),
                  const SizedBox(height: 16),
                  Center(
                    child: ConstrainedBox(constraints: const BoxConstraints(maxWidth: 900), child: sections[_section].$2(value)),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
