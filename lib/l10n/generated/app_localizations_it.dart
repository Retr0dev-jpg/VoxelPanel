// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get appTitle => 'VoxelPanel';

  @override
  String get appTagline => 'Server Minecraft in locale';

  @override
  String get licenseLine => 'Open Source · AGPL-3.0';

  @override
  String get allServers => 'Tutti i server';

  @override
  String get navServers => 'Server';

  @override
  String get homeSubtitle =>
      'Crea, gestisci e avvia i tuoi server Minecraft in locale.';

  @override
  String get searchServers => 'Cerca server...';

  @override
  String get refresh => 'Aggiorna';

  @override
  String get retry => 'Riprova';

  @override
  String get selectServers => 'Seleziona più server';

  @override
  String get selectAll => 'Seleziona tutti';

  @override
  String selectedCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count server selezionati',
      one: '1 server selezionato',
      zero: 'Nessun server selezionato',
    );
    return '$_temp0';
  }

  @override
  String get importServer => 'Importa server';

  @override
  String get newServer => 'Nuovo server';

  @override
  String get noServers =>
      'Nessun server. Creane uno o importa una cartella esistente.';

  @override
  String get noSearchResults => 'Nessun server corrisponde alla ricerca.';

  @override
  String get cancel => 'Annulla';

  @override
  String get confirm => 'Conferma';

  @override
  String get delete => 'Elimina';

  @override
  String get save => 'Salva';

  @override
  String get close => 'Chiudi';

  @override
  String get browse => 'Sfoglia';

  @override
  String get restore => 'Ripristina';

  @override
  String get copied => 'Copiato negli appunti.';

  @override
  String get notAvailable => 'n/d';

  @override
  String get notDetected => 'non rilevato';

  @override
  String get settings => 'Impostazioni';

  @override
  String get settingsSubtitle =>
      'Cartelle locali di VoxelPanel e informazioni sull\'applicazione.';

  @override
  String get settingsLocalData => 'Dati locali';

  @override
  String get settingsLocalDataSubtitle =>
      'Catalogo, server creati da VoxelPanel, runtime Java e backup.';

  @override
  String get settingsAbout => 'Informazioni';

  @override
  String get settingsAboutSubtitle => 'Versione e licenza.';

  @override
  String get openDataFolder => 'Apri cartella dati';

  @override
  String get openServersFolder => 'Apri cartella server';

  @override
  String get openRuntimesFolder => 'Apri runtime Java';

  @override
  String get openBackupsFolder => 'Apri cartella backup';

  @override
  String get windowMinimize => 'Riduci';

  @override
  String get windowMaximize => 'Ingrandisci';

  @override
  String get windowClose => 'Chiudi';

  @override
  String get exitTitle => 'Chiudere VoxelPanel?';

  @override
  String get exitMessage => 'I server avviati da VoxelPanel verranno fermati.';

  @override
  String get exitConfirm => 'Ferma e chiudi';

  @override
  String get statusStopped => 'Fermo';

  @override
  String get statusStarting => 'Avvio';

  @override
  String get statusRunning => 'Online';

  @override
  String get statusStopping => 'Arresto';

  @override
  String get statusCrashed => 'Crash';

  @override
  String get actionStart => 'Avvia';

  @override
  String get actionStop => 'Ferma';

  @override
  String get actionRestart => 'Riavvia';

  @override
  String get openFolder => 'Apri cartella';

  @override
  String get moreActions => 'Altre azioni';

  @override
  String get renameServer => 'Rinomina server';

  @override
  String get deleteServer => 'Elimina server';

  @override
  String serverCardLine(String version, int port, int online, int max) {
    return '$version  ·  :$port  ·  $online/$max giocatori';
  }

  @override
  String serverInfoLine(
    String version,
    String java,
    String ramMin,
    String ramMax,
    int port,
  ) {
    return 'Versione $version  ·  Java $java  ·  RAM $ramMin / $ramMax  ·  Porta $port';
  }

  @override
  String deleteServerTitle(String name) {
    return 'Eliminare $name?';
  }

  @override
  String deleteServersTitle(int count) {
    return 'Eliminare $count server?';
  }

  @override
  String get deleteServerMessage =>
      'Il server viene rimosso dall\'elenco di VoxelPanel. Scegli se eliminare anche i file.';

  @override
  String get deleteServerFiles => 'Elimina anche la cartella del server';

  @override
  String get deleteServerFilesHint =>
      'Mondi, plugin e configurazioni verranno cancellati definitivamente.';

  @override
  String get deleteServerBackups => 'Elimina anche i backup';

  @override
  String get tabOverview => 'Panoramica';

  @override
  String get tabConsole => 'Console';

  @override
  String get tabProperties => 'Proprietà';

  @override
  String get tabPlugins => 'Plugin';

  @override
  String get tabWorlds => 'Mondi';

  @override
  String get tabBackups => 'Backup';

  @override
  String crashBanner(int code) {
    return 'Il server si è chiuso in modo inatteso (codice $code). Controlla la console per i dettagli.';
  }

  @override
  String get eulaMissing =>
      'L\'EULA di Minecraft non è stata accettata: il server non può avviarsi.';

  @override
  String get acceptEula => 'Accetta l\'EULA';

  @override
  String get statPlayers => 'Giocatori';

  @override
  String get noPlayersOnline => 'Nessun giocatore online';

  @override
  String get statCpu => 'CPU';

  @override
  String get statCpuHint => 'Utilizzo del processo';

  @override
  String get statMemory => 'Memoria';

  @override
  String statMemoryHint(String max) {
    return 'Limite $max';
  }

  @override
  String get statUptime => 'Uptime';

  @override
  String get statUptimeHint => 'Dall\'ultimo avvio';

  @override
  String get liveStats => 'Statistiche live';

  @override
  String get liveStatsSubtitle =>
      'Dati letti dal processo del server ogni 2 secondi.';

  @override
  String get serverOnline => 'Server online';

  @override
  String get serverOffline => 'Server offline';

  @override
  String get consoleSearch => 'Cerca nella console';

  @override
  String get logInfo => 'Info';

  @override
  String get logWarn => 'Avvisi';

  @override
  String get logError => 'Errori';

  @override
  String get consoleCopy => 'Copia le righe visibili';

  @override
  String get consoleClear => 'Pulisci la vista';

  @override
  String get consoleScrollEnd => 'Vai in fondo';

  @override
  String get consoleWaiting => 'In attesa di output...';

  @override
  String get consoleEmpty =>
      'Nessun output. Avvia il server per vedere la console.';

  @override
  String get consoleCommandHint =>
      'Comando (frecce su e giù per la cronologia)';

  @override
  String get consoleNotRunning => 'Avvia il server per inviare comandi';

  @override
  String get propertiesSearch => 'Cerca una proprietà';

  @override
  String get propertiesMissing =>
      'server.properties non esiste ancora: verrà creato al primo avvio.';

  @override
  String get propertiesSaved =>
      'Proprietà salvate. Riavvia il server per applicarle.';

  @override
  String get propMotd => 'MOTD';

  @override
  String get propPort => 'Porta';

  @override
  String get propMaxPlayers => 'Giocatori massimi';

  @override
  String get propDifficulty => 'Difficoltà';

  @override
  String get propGamemode => 'Modalità di gioco';

  @override
  String get propViewDistance => 'Distanza di visualizzazione';

  @override
  String get propSimulationDistance => 'Distanza di simulazione';

  @override
  String get propSpawnProtection => 'Protezione dello spawn';

  @override
  String get propLevelName => 'Nome del mondo';

  @override
  String get propLevelSeed => 'Seed';

  @override
  String get propOnlineMode => 'Online mode';

  @override
  String get propWhitelist => 'Whitelist';

  @override
  String get propPvp => 'PvP';

  @override
  String get difficultyPeaceful => 'Pacifica';

  @override
  String get difficultyEasy => 'Facile';

  @override
  String get difficultyNormal => 'Normale';

  @override
  String get difficultyHard => 'Difficile';

  @override
  String get gamemodeSurvival => 'Sopravvivenza';

  @override
  String get gamemodeCreative => 'Creativa';

  @override
  String get gamemodeAdventure => 'Avventura';

  @override
  String get gamemodeSpectator => 'Spettatore';

  @override
  String get installJar => 'Installa jar';

  @override
  String get searchModrinth => 'Cerca su Modrinth';

  @override
  String get stopToEditPlugins =>
      'Ferma il server prima di modificare i plugin.';

  @override
  String get noPlugins => 'Nessun plugin installato.';

  @override
  String get deletePluginTitle => 'Eliminare il plugin?';

  @override
  String get pluginJarTitle => 'Jar del plugin';

  @override
  String get searchPlugins => 'Cerca plugin';

  @override
  String downloadsCount(int count) {
    final intl.NumberFormat countNumberFormat = intl.NumberFormat.compact(
      locale: localeName,
    );
    final String countString = countNumberFormat.format(count);

    return '$countString download';
  }

  @override
  String get noWorlds => 'Nessun mondo. Verrà creato al primo avvio.';

  @override
  String worldActive(String name) {
    return '$name (attivo)';
  }

  @override
  String get setActiveWorld => 'Imposta come mondo attivo';

  @override
  String get restartToApply =>
      'Fatto. Riavvia il server per applicare la modifica.';

  @override
  String deleteWorldTitle(String name) {
    return 'Eliminare il mondo $name?';
  }

  @override
  String get deleteWorldMessage =>
      'La cartella del mondo verrà cancellata definitivamente. Crea prima un backup se vuoi conservarla.';

  @override
  String get createBackup => 'Crea backup';

  @override
  String get backupRunningHint => 'Un backup a server fermo è più coerente.';

  @override
  String get noBackups => 'Nessun backup.';

  @override
  String get restoreBackupTitle => 'Ripristinare il backup?';

  @override
  String get restoreBackupMessage =>
      'I file attuali verranno sovrascritti. Prima del ripristino viene creata automaticamente una copia di sicurezza.';

  @override
  String get deleteBackupTitle => 'Eliminare il backup?';

  @override
  String get createSubtitle =>
      'Crea un nuovo server o importa una cartella già esistente.';

  @override
  String get createMode => 'Crea server';

  @override
  String get createModeSubtitle => 'Configura e avvia un nuovo server';

  @override
  String get importModeSubtitle => 'Importa da una cartella esistente';

  @override
  String get serverFolder => 'Cartella del server';

  @override
  String get serverJarTitle => 'Jar del server';

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
    return 'Plugin: $plugins · Mondi: $worlds';
  }

  @override
  String get eulaCheckbox => 'Accetto l\'EULA di Minecraft (eula=true)';

  @override
  String get eulaHint =>
      'È necessario accettare l\'EULA di Minecraft per creare il server.';

  @override
  String get serverName => 'Nome del server';

  @override
  String get serverNameHint => 'Es. Il mio server';

  @override
  String get destinationFolder => 'Cartella di destinazione';

  @override
  String get destinationFolderHint =>
      'Vuota: VoxelPanel ne crea una nei dati locali';

  @override
  String get installMethod => 'Metodo di installazione';

  @override
  String get methodAutomatic => 'Automatica';

  @override
  String get methodManual => 'Manuale';

  @override
  String get memoryRam => 'Memoria RAM';

  @override
  String get paperVersion => 'Versione Paper';

  @override
  String latestVersion(String version) {
    return '$version (ultima)';
  }

  @override
  String get installedJava => 'Runtime Java installato';

  @override
  String get downloadOtherJava => 'Scarica un altro Java';

  @override
  String get noLocalJar => 'Nessun jar locale';

  @override
  String get chooseJar => 'Scegli jar';

  @override
  String get ramMin => 'RAM minima';

  @override
  String get ramMax => 'RAM massima';

  @override
  String get jvmFlags => 'Flag JVM';

  @override
  String get javaVersion => 'Versione Java';

  @override
  String get installing => 'Installazione...';

  @override
  String get createServer => 'Crea server';

  @override
  String get issueMissingName => 'Inserisci un nome.';

  @override
  String get issueEula => 'Accetta l\'EULA di Minecraft per continuare.';

  @override
  String get issueMissingVersion => 'Seleziona una versione.';

  @override
  String get issueMissingVersionOrJar =>
      'Seleziona una versione oppure un jar.';

  @override
  String get issueMissingJava => 'Seleziona un runtime Java.';

  @override
  String get errorServerRunning => 'Ferma il server prima di continuare.';

  @override
  String get errorServerStopped => 'Il server non è in esecuzione.';

  @override
  String get errorEulaRequired =>
      'Accetta l\'EULA di Minecraft prima di avviare il server.';

  @override
  String errorNetwork(String details) {
    return 'Errore di rete: $details';
  }
}
