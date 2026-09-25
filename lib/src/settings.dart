// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/src/rust/api/settings.dart';

export 'package:voxel_panel/src/rust/api/settings.dart';

/// Launcher settings persisted by Rust in `settings.json`.
final launcherSettingsProvider = AsyncNotifierProvider<LauncherSettingsNotifier, LauncherSettings>(LauncherSettingsNotifier.new);

class LauncherSettingsNotifier extends AsyncNotifier<LauncherSettings> {
  @override
  Future<LauncherSettings> build() => getLauncherSettings();

  /// Saves and returns the normalized settings; errors are rethrown for the caller to show.
  Future<LauncherSettings> save(LauncherSettings settings) async {
    final saved = await saveLauncherSettings(settings: settings);
    state = AsyncData(saved);
    return saved;
  }

  void replace(LauncherSettings settings) => state = AsyncData(settings);
}

/// Settings available synchronously once loaded; before that the defaults of the widgets apply.
final settingsValueProvider = Provider<LauncherSettings?>((ref) => ref.watch(launcherSettingsProvider).value);

ThemeMode themeModeOf(ThemePreference preference) => switch (preference) {
  ThemePreference.system => ThemeMode.system,
  ThemePreference.light => ThemeMode.light,
  ThemePreference.dark => ThemeMode.dark,
};

Locale? localeOf(String language) => language == 'system' ? null : Locale(language);

extension LauncherSettingsCopy on LauncherSettings {
  LauncherSettings copyWith({
    GeneralSettings? general,
    AppearanceSettings? appearance,
    PathSettings? paths,
    JavaSettings? java,
    DefaultsSettings? defaults,
    ConsoleSettings? console,
    BackupSettings? backup,
    NetworkSettings? network,
    NotificationSettings? notifications,
    AdvancedSettings? advanced,
  }) => LauncherSettings(
    schemaVersion: schemaVersion,
    general: general ?? this.general,
    appearance: appearance ?? this.appearance,
    paths: paths ?? this.paths,
    java: java ?? this.java,
    defaults: defaults ?? this.defaults,
    console: console ?? this.console,
    backup: backup ?? this.backup,
    network: network ?? this.network,
    notifications: notifications ?? this.notifications,
    advanced: advanced ?? this.advanced,
  );
}

extension GeneralSettingsCopy on GeneralSettings {
  GeneralSettings copyWith({
    String? language,
    bool? checkUpdates,
    bool? launchAtStartup,
    CloseBehavior? closeBehavior,
    bool? autostartServers,
    int? stopTimeoutSecs,
  }) => GeneralSettings(
    language: language ?? this.language,
    checkUpdates: checkUpdates ?? this.checkUpdates,
    launchAtStartup: launchAtStartup ?? this.launchAtStartup,
    closeBehavior: closeBehavior ?? this.closeBehavior,
    autostartServers: autostartServers ?? this.autostartServers,
    stopTimeoutSecs: stopTimeoutSecs ?? this.stopTimeoutSecs,
  );
}

extension AppearanceSettingsCopy on AppearanceSettings {
  AppearanceSettings copyWith({ThemePreference? theme, int? accentColor, double? textScale, bool? compact}) => AppearanceSettings(
    theme: theme ?? this.theme,
    accentColor: accentColor ?? this.accentColor,
    textScale: textScale ?? this.textScale,
    compact: compact ?? this.compact,
  );
}

extension DefaultsSettingsCopy on DefaultsSettings {
  DefaultsSettings copyWith({String? ramMin, String? ramMax, JvmPreset? jvmPreset, int? port, String? provider}) => DefaultsSettings(
    ramMin: ramMin ?? this.ramMin,
    ramMax: ramMax ?? this.ramMax,
    jvmPreset: jvmPreset ?? this.jvmPreset,
    port: port ?? this.port,
    provider: provider ?? this.provider,
  );
}

extension ConsoleSettingsCopy on ConsoleSettings {
  ConsoleSettings copyWith({int? maxLines, bool? timestamps, double? fontSize, bool? wrap}) => ConsoleSettings(
    maxLines: maxLines ?? this.maxLines,
    timestamps: timestamps ?? this.timestamps,
    fontSize: fontSize ?? this.fontSize,
    wrap: wrap ?? this.wrap,
  );
}

extension BackupSettingsCopy on BackupSettings {
  BackupSettings copyWith({int? retention, int? compressionLevel, List<String>? exclusions}) => BackupSettings(
    retention: retention ?? this.retention,
    compressionLevel: compressionLevel ?? this.compressionLevel,
    exclusions: exclusions ?? this.exclusions,
  );
}

extension NetworkSettingsCopy on NetworkSettings {
  NetworkSettings copyWith({String? proxy, int? parallelDownloads, int? timeoutSecs, String? curseforgeApiKey}) => NetworkSettings(
    proxy: proxy ?? this.proxy,
    parallelDownloads: parallelDownloads ?? this.parallelDownloads,
    timeoutSecs: timeoutSecs ?? this.timeoutSecs,
    curseforgeApiKey: curseforgeApiKey ?? this.curseforgeApiKey,
  );
}

extension NotificationSettingsCopy on NotificationSettings {
  NotificationSettings copyWith({bool? crash, bool? ready, bool? playerJoin}) => NotificationSettings(
    crash: crash ?? this.crash,
    ready: ready ?? this.ready,
    playerJoin: playerJoin ?? this.playerJoin,
  );
}

extension AdvancedSettingsCopy on AdvancedSettings {
  AdvancedSettings copyWith({bool? logging, AppLogLevel? logLevel}) => AdvancedSettings(logging: logging ?? this.logging, logLevel: logLevel ?? this.logLevel);
}
