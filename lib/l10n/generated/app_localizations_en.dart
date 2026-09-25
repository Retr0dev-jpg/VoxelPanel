// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'VoxelPanel';

  @override
  String get appTagline => 'Local Minecraft servers';

  @override
  String get licenseLine => 'Open Source · AGPL-3.0';

  @override
  String get allServers => 'All servers';

  @override
  String get navServers => 'Servers';

  @override
  String get homeSubtitle =>
      'Create, manage and run your Minecraft servers locally.';

  @override
  String get searchServers => 'Search servers...';

  @override
  String get refresh => 'Refresh';

  @override
  String get retry => 'Retry';

  @override
  String get selectServers => 'Select multiple servers';

  @override
  String get selectAll => 'Select all';

  @override
  String selectedCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count servers selected',
      one: '1 server selected',
      zero: 'No server selected',
    );
    return '$_temp0';
  }

  @override
  String get importServer => 'Import server';

  @override
  String get newServer => 'New server';

  @override
  String get noServers =>
      'No servers yet. Create one or import an existing folder.';

  @override
  String get noSearchResults => 'No server matches your search.';

  @override
  String get cancel => 'Cancel';

  @override
  String get confirm => 'Confirm';

  @override
  String get delete => 'Delete';

  @override
  String get save => 'Save';

  @override
  String get close => 'Close';

  @override
  String get browse => 'Browse';

  @override
  String get restore => 'Restore';

  @override
  String get copied => 'Copied to clipboard.';

  @override
  String get notAvailable => 'n/a';

  @override
  String get notDetected => 'not detected';

  @override
  String get settings => 'Settings';

  @override
  String get settingsSubtitle =>
      'VoxelPanel local folders and application details.';

  @override
  String get settingsLocalData => 'Local data';

  @override
  String get settingsLocalDataSubtitle =>
      'Catalog, servers created by VoxelPanel, Java runtimes and backups.';

  @override
  String get settingsAbout => 'About';

  @override
  String get settingsAboutSubtitle => 'Version and license.';

  @override
  String get openDataFolder => 'Open data folder';

  @override
  String get openServersFolder => 'Open servers folder';

  @override
  String get openRuntimesFolder => 'Open Java runtimes';

  @override
  String get openBackupsFolder => 'Open backups folder';

  @override
  String get windowMinimize => 'Minimize';

  @override
  String get windowMaximize => 'Maximize';

  @override
  String get windowClose => 'Close';

  @override
  String get exitTitle => 'Close VoxelPanel?';

  @override
  String get exitMessage => 'Servers started by VoxelPanel will be stopped.';

  @override
  String get exitConfirm => 'Stop and close';

  @override
  String get statusStopped => 'Stopped';

  @override
  String get statusStarting => 'Starting';

  @override
  String get statusRunning => 'Online';

  @override
  String get statusStopping => 'Stopping';

  @override
  String get statusCrashed => 'Crashed';

  @override
  String get actionStart => 'Start';

  @override
  String get actionStop => 'Stop';

  @override
  String get actionRestart => 'Restart';

  @override
  String get openFolder => 'Open folder';

  @override
  String get moreActions => 'More actions';

  @override
  String get renameServer => 'Rename server';

  @override
  String get deleteServer => 'Delete server';

  @override
  String serverCardLine(String version, int port, int online, int max) {
    return '$version  ·  :$port  ·  $online/$max players';
  }

  @override
  String serverInfoLine(
    String version,
    String java,
    String ramMin,
    String ramMax,
    int port,
  ) {
    return 'Version $version  ·  Java $java  ·  RAM $ramMin / $ramMax  ·  Port $port';
  }

  @override
  String deleteServerTitle(String name) {
    return 'Delete $name?';
  }

  @override
  String deleteServersTitle(int count) {
    return 'Delete $count servers?';
  }

  @override
  String get deleteServerMessage =>
      'The server is removed from VoxelPanel. Choose whether to delete its files too.';

  @override
  String get deleteServerFiles => 'Also delete the server folder';

  @override
  String get deleteServerFilesHint =>
      'Worlds, plugins and configuration will be permanently deleted.';

  @override
  String get deleteServerBackups => 'Also delete backups';

  @override
  String get tabOverview => 'Overview';

  @override
  String get tabConsole => 'Console';

  @override
  String get tabProperties => 'Properties';

  @override
  String get tabPlugins => 'Plugins';

  @override
  String get tabWorlds => 'Worlds';

  @override
  String get tabBackups => 'Backups';

  @override
  String crashBanner(int code) {
    return 'The server exited unexpectedly (code $code). Check the console for details.';
  }

  @override
  String get eulaMissing =>
      'The Minecraft EULA has not been accepted: the server cannot start.';

  @override
  String get acceptEula => 'Accept EULA';

  @override
  String get statPlayers => 'Players';

  @override
  String get noPlayersOnline => 'No players online';

  @override
  String get statCpu => 'CPU';

  @override
  String get statCpuHint => 'Process usage';

  @override
  String get statMemory => 'Memory';

  @override
  String statMemoryHint(String max) {
    return 'Limit $max';
  }

  @override
  String get statUptime => 'Uptime';

  @override
  String get statUptimeHint => 'Since last start';

  @override
  String get liveStats => 'Live stats';

  @override
  String get liveStatsSubtitle =>
      'Read from the server process every 2 seconds.';

  @override
  String get serverOnline => 'Server online';

  @override
  String get serverOffline => 'Server offline';

  @override
  String get consoleSearch => 'Search console';

  @override
  String get logInfo => 'Info';

  @override
  String get logWarn => 'Warnings';

  @override
  String get logError => 'Errors';

  @override
  String get consoleCopy => 'Copy visible lines';

  @override
  String get consoleClear => 'Clear view';

  @override
  String get consoleScrollEnd => 'Scroll to bottom';

  @override
  String get consoleWaiting => 'Waiting for output...';

  @override
  String get consoleEmpty => 'No output. Start the server to see the console.';

  @override
  String get consoleCommandHint => 'Command (up and down arrows for history)';

  @override
  String get consoleNotRunning => 'Start the server to send commands';

  @override
  String get propertiesSearch => 'Search a property';

  @override
  String get propertiesMissing =>
      'server.properties does not exist yet: it will be created on first start.';

  @override
  String get propertiesSaved =>
      'Properties saved. Restart the server to apply them.';

  @override
  String get propMotd => 'MOTD';

  @override
  String get propPort => 'Port';

  @override
  String get propMaxPlayers => 'Max players';

  @override
  String get propDifficulty => 'Difficulty';

  @override
  String get propGamemode => 'Game mode';

  @override
  String get propViewDistance => 'View distance';

  @override
  String get propSimulationDistance => 'Simulation distance';

  @override
  String get propSpawnProtection => 'Spawn protection';

  @override
  String get propLevelName => 'World name';

  @override
  String get propLevelSeed => 'Seed';

  @override
  String get propOnlineMode => 'Online mode';

  @override
  String get propWhitelist => 'Whitelist';

  @override
  String get propPvp => 'PvP';

  @override
  String get difficultyPeaceful => 'Peaceful';

  @override
  String get difficultyEasy => 'Easy';

  @override
  String get difficultyNormal => 'Normal';

  @override
  String get difficultyHard => 'Hard';

  @override
  String get gamemodeSurvival => 'Survival';

  @override
  String get gamemodeCreative => 'Creative';

  @override
  String get gamemodeAdventure => 'Adventure';

  @override
  String get gamemodeSpectator => 'Spectator';

  @override
  String get installJar => 'Install jar';

  @override
  String get searchModrinth => 'Search Modrinth';

  @override
  String get stopToEditPlugins => 'Stop the server before changing plugins.';

  @override
  String get noPlugins => 'No plugins installed.';

  @override
  String get deletePluginTitle => 'Delete plugin?';

  @override
  String get pluginJarTitle => 'Plugin jar';

  @override
  String get searchPlugins => 'Search plugins';

  @override
  String downloadsCount(int count) {
    final intl.NumberFormat countNumberFormat = intl.NumberFormat.compact(
      locale: localeName,
    );
    final String countString = countNumberFormat.format(count);

    return '$countString downloads';
  }

  @override
  String get noWorlds => 'No worlds. One will be created on first start.';

  @override
  String worldActive(String name) {
    return '$name (active)';
  }

  @override
  String get setActiveWorld => 'Set as active world';

  @override
  String get restartToApply => 'Done. Restart the server to apply the change.';

  @override
  String deleteWorldTitle(String name) {
    return 'Delete world $name?';
  }

  @override
  String get deleteWorldMessage =>
      'The world folder will be permanently deleted. Create a backup first if you want to keep it.';

  @override
  String get createBackup => 'Create backup';

  @override
  String get backupRunningHint =>
      'A backup taken while stopped is more consistent.';

  @override
  String get noBackups => 'No backups.';

  @override
  String get restoreBackupTitle => 'Restore backup?';

  @override
  String get restoreBackupMessage =>
      'Current files will be overwritten. A safety copy is created automatically before restoring.';

  @override
  String get deleteBackupTitle => 'Delete backup?';

  @override
  String get createSubtitle =>
      'Create a new server or import an existing folder.';

  @override
  String get createMode => 'Create server';

  @override
  String get createModeSubtitle => 'Configure and run a new server';

  @override
  String get importModeSubtitle => 'Import from an existing folder';

  @override
  String get serverFolder => 'Server folder';

  @override
  String get serverJarTitle => 'Server jar';

  @override
  String importDetectedPaper(String value) {
    return 'Paper: $value';
  }

  @override
  String importDetectedJava(String value) {
    return 'Java: $value';
  }

  @override
  String importDetectedRam(String min, String max) {
    return 'RAM: $min / $max';
  }

  @override
  String importDetectedContent(int plugins, int worlds) {
    return 'Plugins: $plugins · Worlds: $worlds';
  }

  @override
  String get eulaCheckbox => 'I accept the Minecraft EULA (eula=true)';

  @override
  String get eulaHint =>
      'You must accept the Minecraft EULA to create the server.';

  @override
  String get serverName => 'Server name';

  @override
  String get serverNameHint => 'E.g. My server';

  @override
  String get destinationFolder => 'Destination folder';

  @override
  String get destinationFolderHint =>
      'Empty: VoxelPanel creates one in its local data';

  @override
  String get installMethod => 'Installation method';

  @override
  String get methodAutomatic => 'Automatic';

  @override
  String get methodManual => 'Manual';

  @override
  String get memoryRam => 'RAM';

  @override
  String get paperVersion => 'Paper version';

  @override
  String latestVersion(String version) {
    return '$version (latest)';
  }

  @override
  String get installedJava => 'Installed Java runtime';

  @override
  String get downloadOtherJava => 'Download another Java';

  @override
  String get noLocalJar => 'No local jar';

  @override
  String get chooseJar => 'Choose jar';

  @override
  String get ramMin => 'Minimum RAM';

  @override
  String get ramMax => 'Maximum RAM';

  @override
  String get jvmFlags => 'JVM flags';

  @override
  String get javaVersion => 'Java version';

  @override
  String get installing => 'Installing...';

  @override
  String get createServer => 'Create server';

  @override
  String get issueMissingName => 'Enter a name.';

  @override
  String get issueEula => 'Accept the Minecraft EULA to continue.';

  @override
  String get issueMissingVersion => 'Select a version.';

  @override
  String get issueMissingVersionOrJar => 'Select a version or a jar.';

  @override
  String get issueMissingJava => 'Select a Java runtime.';

  @override
  String get errorServerRunning => 'Stop the server before continuing.';

  @override
  String get errorServerStopped => 'The server is not running.';

  @override
  String get errorEulaRequired =>
      'Accept the Minecraft EULA before starting the server.';

  @override
  String errorNetwork(String details) {
    return 'Network error: $details';
  }
}
