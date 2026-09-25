// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/settings/launcher_settings_screen.dart';
import 'package:voxel_panel/screens/settings/settings_widgets.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';

const accentChoices = <int>[0xFF7C4DFF, 0xFF2979FF, 0xFF00BFA5, 0xFF43A047, 0xFFFFA000, 0xFFF4511E, 0xFFE91E63, 0xFF8D6E63];

class GeneralSection extends ConsumerWidget {
  const GeneralSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final general = settings.general;
    void save(GeneralSettings next) => saveSettings(context, ref, settings.copyWith(general: next));
    return Column(
      children: [
        SettingsGroup(
          title: l.settingsGeneral,
          children: [
            SettingsRow(
              title: l.settingLanguage,
              child: SettingsDropdown<String>(
                value: general.language,
                items: {'system': l.languageSystem, 'it': 'Italiano', 'en': 'English'},
                onChanged: (value) => save(general.copyWith(language: value)),
              ),
            ),
            SettingsSwitch(
              title: l.settingCheckUpdates,
              subtitle: l.settingCheckUpdatesHint,
              value: general.checkUpdates,
              onChanged: (value) => save(general.copyWith(checkUpdates: value)),
            ),
            SettingsSwitch(
              title: l.settingLaunchAtStartup,
              value: general.launchAtStartup,
              onChanged: (value) => save(general.copyWith(launchAtStartup: value)),
            ),
          ],
        ),
        SettingsGroup(
          title: l.settingsWindowAndServers,
          children: [
            SettingsRow(
              title: l.settingCloseBehavior,
              child: SettingsDropdown<CloseBehavior>(
                value: general.closeBehavior,
                items: {
                  CloseBehavior.ask: l.closeAsk,
                  CloseBehavior.stopAndExit: l.closeStop,
                  CloseBehavior.minimizeToTray: l.closeTray,
                },
                onChanged: (value) => save(general.copyWith(closeBehavior: value)),
              ),
            ),
            SettingsSwitch(
              title: l.settingAutostartServers,
              subtitle: l.settingAutostartServersHint,
              value: general.autostartServers,
              onChanged: (value) => save(general.copyWith(autostartServers: value)),
            ),
            SettingsRow(
              title: l.settingStopTimeout,
              subtitle: l.settingStopTimeoutHint,
              child: CommitTextField(
                value: '${general.stopTimeoutSecs}',
                numeric: true,
                suffix: 's',
                onCommit: (value) => save(general.copyWith(stopTimeoutSecs: int.tryParse(value) ?? general.stopTimeoutSecs)),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class AppearanceSection extends ConsumerWidget {
  const AppearanceSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final appearance = settings.appearance;
    void save(AppearanceSettings next) => saveSettings(context, ref, settings.copyWith(appearance: next));
    return SettingsGroup(
      title: l.settingsAppearance,
      children: [
        SettingsRow(
          title: l.settingTheme,
          child: SegmentedButton<ThemePreference>(
            segments: [
              ButtonSegment(value: ThemePreference.system, label: Text(l.themeSystem)),
              ButtonSegment(value: ThemePreference.light, label: Text(l.themeLight)),
              ButtonSegment(value: ThemePreference.dark, label: Text(l.themeDark)),
            ],
            selected: {appearance.theme},
            onSelectionChanged: (value) => save(appearance.copyWith(theme: value.first)),
          ),
        ),
        SettingsRow(
          title: l.settingAccent,
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final color in accentChoices)
                InkWell(
                  borderRadius: BorderRadius.circular(18),
                  onTap: () => save(appearance.copyWith(accentColor: color)),
                  child: Container(
                    width: 30,
                    height: 30,
                    decoration: BoxDecoration(
                      color: Color(color),
                      shape: BoxShape.circle,
                      border: Border.all(color: appearance.accentColor == color ? Theme.of(context).colorScheme.onSurface : Colors.transparent, width: 3),
                    ),
                  ),
                ),
            ],
          ),
        ),
        SettingsRow(
          title: l.settingTextScale,
          subtitle: '${(appearance.textScale * 100).round()}%',
          child: Slider(
            value: appearance.textScale,
            min: 0.8,
            max: 1.6,
            divisions: 8,
            label: '${(appearance.textScale * 100).round()}%',
            onChanged: (value) => ref.read(launcherSettingsProvider.notifier).replace(settings.copyWith(appearance: appearance.copyWith(textScale: value))),
            onChangeEnd: (value) => save(appearance.copyWith(textScale: value)),
          ),
        ),
        SettingsSwitch(
          title: l.settingCompact,
          subtitle: l.settingCompactHint,
          value: appearance.compact,
          onChanged: (value) => save(appearance.copyWith(compact: value)),
        ),
        SettingsRow(
          title: l.settingContentLayout,
          subtitle: l.settingContentLayoutHint,
          child: SettingsDropdown<ContentLayout>(
            value: appearance.contentLayout,
            items: {ContentLayout.list: l.layoutList, ContentLayout.grid: l.layoutGrid},
            onChanged: (value) => save(appearance.copyWith(contentLayout: value)),
          ),
        ),
      ],
    );
  }
}

class DefaultsSection extends ConsumerWidget {
  const DefaultsSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final defaults = settings.defaults;
    final ram = {'': l.ramAutomatic, for (final choice in ramPresets()) choice.value: choice.label};
    final presets = {for (final preset in jvmPresets()) preset.preset: preset};
    void save(DefaultsSettings next) => saveSettings(context, ref, settings.copyWith(defaults: next));
    return SettingsGroup(
      title: l.settingsDefaults,
      subtitle: l.settingsDefaultsHint,
      children: [
        SettingsRow(
          title: l.ramMin,
          child: SettingsDropdown<String>(value: defaults.ramMin, items: ram, onChanged: (value) => save(defaults.copyWith(ramMin: value))),
        ),
        SettingsRow(
          title: l.ramMax,
          child: SettingsDropdown<String>(value: defaults.ramMax, items: ram, onChanged: (value) => save(defaults.copyWith(ramMax: value))),
        ),
        SettingsRow(
          title: l.settingJvmPreset,
          subtitle: presets[defaults.jvmPreset]?.flags.isEmpty ?? true ? l.jvmPresetNoneHint : l.jvmPresetFlags(presets[defaults.jvmPreset]!.flags.length),
          child: SettingsDropdown<JvmPreset>(
            value: defaults.jvmPreset,
            items: {JvmPreset.aikar: l.jvmPresetAikar, JvmPreset.g1: l.jvmPresetG1, JvmPreset.zgc: l.jvmPresetZgc, JvmPreset.none: l.jvmPresetNone},
            onChanged: (value) => save(defaults.copyWith(jvmPreset: value)),
          ),
        ),
        SettingsRow(
          title: l.settingFirstPort,
          subtitle: l.settingFirstPortHint,
          child: CommitTextField(value: '${defaults.port}', numeric: true, onCommit: (value) => save(defaults.copyWith(port: int.tryParse(value) ?? defaults.port))),
        ),
      ],
    );
  }
}

class ConsoleSection extends ConsumerWidget {
  const ConsoleSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final console = settings.console;
    void save(ConsoleSettings next) => saveSettings(context, ref, settings.copyWith(console: next));
    return SettingsGroup(
      title: l.settingsConsole,
      children: [
        SettingsRow(
          title: l.settingMaxLines,
          subtitle: l.settingMaxLinesHint,
          child: CommitTextField(value: '${console.maxLines}', numeric: true, onCommit: (value) => save(console.copyWith(maxLines: int.tryParse(value) ?? console.maxLines))),
        ),
        SettingsRow(
          title: l.settingFontSize,
          subtitle: '${console.fontSize.round()} px',
          child: Slider(
            value: console.fontSize,
            min: 9,
            max: 24,
            divisions: 15,
            label: '${console.fontSize.round()}',
            onChanged: (value) => ref.read(launcherSettingsProvider.notifier).replace(settings.copyWith(console: console.copyWith(fontSize: value))),
            onChangeEnd: (value) => save(console.copyWith(fontSize: value)),
          ),
        ),
        SettingsSwitch(title: l.settingTimestamps, subtitle: l.settingTimestampsHint, value: console.timestamps, onChanged: (value) => save(console.copyWith(timestamps: value))),
        SettingsSwitch(title: l.settingWrap, value: console.wrap, onChanged: (value) => save(console.copyWith(wrap: value))),
      ],
    );
  }
}

class BackupSection extends ConsumerWidget {
  const BackupSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final backup = settings.backup;
    void save(BackupSettings next) => saveSettings(context, ref, settings.copyWith(backup: next));
    return SettingsGroup(
      title: l.settingsBackup,
      children: [
        SettingsRow(
          title: l.settingRetention,
          subtitle: l.settingRetentionHint,
          child: CommitTextField(value: '${backup.retention}', numeric: true, onCommit: (value) => save(backup.copyWith(retention: int.tryParse(value) ?? backup.retention))),
        ),
        SettingsRow(
          title: l.settingCompression,
          subtitle: backup.compressionLevel == 0 ? l.compressionNone : '${backup.compressionLevel} / 9',
          child: Slider(
            value: backup.compressionLevel.toDouble(),
            min: 0,
            max: 9,
            divisions: 9,
            label: '${backup.compressionLevel}',
            onChanged: (value) => ref.read(launcherSettingsProvider.notifier).replace(settings.copyWith(backup: backup.copyWith(compressionLevel: value.round()))),
            onChangeEnd: (value) => save(backup.copyWith(compressionLevel: value.round())),
          ),
        ),
        SettingsRow(
          title: l.settingExclusions,
          subtitle: l.settingExclusionsHint,
          child: CommitTextField(
            value: backup.exclusions.join(', '),
            hint: 'cache, logs, *.lock',
            onCommit: (value) => save(backup.copyWith(exclusions: [for (final part in value.split(',')) if (part.trim().isNotEmpty) part.trim()])),
          ),
        ),
      ],
    );
  }
}

class NetworkSection extends ConsumerWidget {
  const NetworkSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final network = settings.network;
    void save(NetworkSettings next) => saveSettings(context, ref, settings.copyWith(network: next));
    return Column(
      children: [
        SettingsGroup(
          title: l.settingsNetwork,
          children: [
            SettingsRow(
              title: l.settingProxy,
              subtitle: l.settingProxyHint,
              child: CommitTextField(value: network.proxy, hint: 'http://proxy:8080', onCommit: (value) => save(network.copyWith(proxy: value))),
            ),
            SettingsRow(
              title: l.settingParallelDownloads,
              child: SettingsDropdown<int>(
                value: network.parallelDownloads,
                items: {for (final value in [1, 2, 4, 6, 8, 12, 16]) value: '$value'},
                onChanged: (value) => save(network.copyWith(parallelDownloads: value)),
              ),
            ),
            SettingsRow(
              title: l.settingTimeout,
              child: CommitTextField(value: '${network.timeoutSecs}', numeric: true, suffix: 's', onCommit: (value) => save(network.copyWith(timeoutSecs: int.tryParse(value) ?? network.timeoutSecs))),
            ),
          ],
        ),
        SettingsGroup(
          title: 'CurseForge',
          subtitle: l.settingCurseforgeHint,
          children: [
            SettingsRow(
              title: l.settingCurseforgeKey,
              child: CommitTextField(value: network.curseforgeApiKey, obscure: true, onCommit: (value) => save(network.copyWith(curseforgeApiKey: value))),
            ),
          ],
        ),
      ],
    );
  }
}

class NotificationsSection extends ConsumerWidget {
  const NotificationsSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final notifications = settings.notifications;
    void save(NotificationSettings next) => saveSettings(context, ref, settings.copyWith(notifications: next));
    return SettingsGroup(
      title: l.settingsNotifications,
      subtitle: l.settingsNotificationsHint,
      children: [
        SettingsSwitch(title: l.notifyOnCrash, value: notifications.crash, onChanged: (value) => save(notifications.copyWith(crash: value))),
        SettingsSwitch(title: l.notifyOnReady, value: notifications.ready, onChanged: (value) => save(notifications.copyWith(ready: value))),
        SettingsSwitch(title: l.notifyOnPlayer, value: notifications.playerJoin, onChanged: (value) => save(notifications.copyWith(playerJoin: value))),
        const SizedBox(height: 4),
        Text(l.notificationsSystemHint, style: TextStyle(color: context.voxel.muted, fontSize: 12)),
      ],
    );
  }
}
