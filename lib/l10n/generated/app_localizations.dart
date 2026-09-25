import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_it.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'generated/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('it'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In it, this message translates to:
  /// **'VoxelPanel'**
  String get appTitle;

  /// No description provided for @appTagline.
  ///
  /// In it, this message translates to:
  /// **'Server Minecraft in locale'**
  String get appTagline;

  /// No description provided for @licenseLine.
  ///
  /// In it, this message translates to:
  /// **'Open Source · AGPL-3.0'**
  String get licenseLine;

  /// No description provided for @allServers.
  ///
  /// In it, this message translates to:
  /// **'Tutti i server'**
  String get allServers;

  /// No description provided for @navServers.
  ///
  /// In it, this message translates to:
  /// **'Server'**
  String get navServers;

  /// No description provided for @homeSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Crea, gestisci e avvia i tuoi server Minecraft in locale.'**
  String get homeSubtitle;

  /// No description provided for @searchServers.
  ///
  /// In it, this message translates to:
  /// **'Cerca server...'**
  String get searchServers;

  /// No description provided for @refresh.
  ///
  /// In it, this message translates to:
  /// **'Aggiorna'**
  String get refresh;

  /// No description provided for @retry.
  ///
  /// In it, this message translates to:
  /// **'Riprova'**
  String get retry;

  /// No description provided for @selectServers.
  ///
  /// In it, this message translates to:
  /// **'Seleziona più server'**
  String get selectServers;

  /// No description provided for @selectAll.
  ///
  /// In it, this message translates to:
  /// **'Seleziona tutti'**
  String get selectAll;

  /// No description provided for @selectedCount.
  ///
  /// In it, this message translates to:
  /// **'{count, plural, =0{Nessun server selezionato} =1{1 server selezionato} other{{count} server selezionati}}'**
  String selectedCount(int count);

  /// No description provided for @importServer.
  ///
  /// In it, this message translates to:
  /// **'Importa server'**
  String get importServer;

  /// No description provided for @newServer.
  ///
  /// In it, this message translates to:
  /// **'Nuovo server'**
  String get newServer;

  /// No description provided for @noServers.
  ///
  /// In it, this message translates to:
  /// **'Nessun server. Creane uno o importa una cartella esistente.'**
  String get noServers;

  /// No description provided for @noSearchResults.
  ///
  /// In it, this message translates to:
  /// **'Nessun server corrisponde alla ricerca.'**
  String get noSearchResults;

  /// No description provided for @cancel.
  ///
  /// In it, this message translates to:
  /// **'Annulla'**
  String get cancel;

  /// No description provided for @confirm.
  ///
  /// In it, this message translates to:
  /// **'Conferma'**
  String get confirm;

  /// No description provided for @delete.
  ///
  /// In it, this message translates to:
  /// **'Elimina'**
  String get delete;

  /// No description provided for @save.
  ///
  /// In it, this message translates to:
  /// **'Salva'**
  String get save;

  /// No description provided for @close.
  ///
  /// In it, this message translates to:
  /// **'Chiudi'**
  String get close;

  /// No description provided for @browse.
  ///
  /// In it, this message translates to:
  /// **'Sfoglia'**
  String get browse;

  /// No description provided for @restore.
  ///
  /// In it, this message translates to:
  /// **'Ripristina'**
  String get restore;

  /// No description provided for @copied.
  ///
  /// In it, this message translates to:
  /// **'Copiato negli appunti.'**
  String get copied;

  /// No description provided for @notAvailable.
  ///
  /// In it, this message translates to:
  /// **'n/d'**
  String get notAvailable;

  /// No description provided for @notDetected.
  ///
  /// In it, this message translates to:
  /// **'non rilevato'**
  String get notDetected;

  /// No description provided for @settings.
  ///
  /// In it, this message translates to:
  /// **'Impostazioni'**
  String get settings;

  /// No description provided for @settingsSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Cartelle locali di VoxelPanel e informazioni sull\'applicazione.'**
  String get settingsSubtitle;

  /// No description provided for @settingsLocalData.
  ///
  /// In it, this message translates to:
  /// **'Dati locali'**
  String get settingsLocalData;

  /// No description provided for @settingsLocalDataSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Catalogo, server creati da VoxelPanel, runtime Java e backup.'**
  String get settingsLocalDataSubtitle;

  /// No description provided for @settingsAbout.
  ///
  /// In it, this message translates to:
  /// **'Informazioni'**
  String get settingsAbout;

  /// No description provided for @settingsAboutSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Versione e licenza.'**
  String get settingsAboutSubtitle;

  /// No description provided for @openDataFolder.
  ///
  /// In it, this message translates to:
  /// **'Apri cartella dati'**
  String get openDataFolder;

  /// No description provided for @openServersFolder.
  ///
  /// In it, this message translates to:
  /// **'Apri cartella server'**
  String get openServersFolder;

  /// No description provided for @openRuntimesFolder.
  ///
  /// In it, this message translates to:
  /// **'Apri runtime Java'**
  String get openRuntimesFolder;

  /// No description provided for @openBackupsFolder.
  ///
  /// In it, this message translates to:
  /// **'Apri cartella backup'**
  String get openBackupsFolder;

  /// No description provided for @windowMinimize.
  ///
  /// In it, this message translates to:
  /// **'Riduci'**
  String get windowMinimize;

  /// No description provided for @windowMaximize.
  ///
  /// In it, this message translates to:
  /// **'Ingrandisci'**
  String get windowMaximize;

  /// No description provided for @windowClose.
  ///
  /// In it, this message translates to:
  /// **'Chiudi'**
  String get windowClose;

  /// No description provided for @exitTitle.
  ///
  /// In it, this message translates to:
  /// **'Chiudere VoxelPanel?'**
  String get exitTitle;

  /// No description provided for @exitMessage.
  ///
  /// In it, this message translates to:
  /// **'I server avviati da VoxelPanel verranno fermati.'**
  String get exitMessage;

  /// No description provided for @exitConfirm.
  ///
  /// In it, this message translates to:
  /// **'Ferma e chiudi'**
  String get exitConfirm;

  /// No description provided for @statusStopped.
  ///
  /// In it, this message translates to:
  /// **'Fermo'**
  String get statusStopped;

  /// No description provided for @statusStarting.
  ///
  /// In it, this message translates to:
  /// **'Avvio'**
  String get statusStarting;

  /// No description provided for @statusRunning.
  ///
  /// In it, this message translates to:
  /// **'Online'**
  String get statusRunning;

  /// No description provided for @statusStopping.
  ///
  /// In it, this message translates to:
  /// **'Arresto'**
  String get statusStopping;

  /// No description provided for @statusCrashed.
  ///
  /// In it, this message translates to:
  /// **'Crash'**
  String get statusCrashed;

  /// No description provided for @actionStart.
  ///
  /// In it, this message translates to:
  /// **'Avvia'**
  String get actionStart;

  /// No description provided for @actionStop.
  ///
  /// In it, this message translates to:
  /// **'Ferma'**
  String get actionStop;

  /// No description provided for @actionRestart.
  ///
  /// In it, this message translates to:
  /// **'Riavvia'**
  String get actionRestart;

  /// No description provided for @openFolder.
  ///
  /// In it, this message translates to:
  /// **'Apri cartella'**
  String get openFolder;

  /// No description provided for @moreActions.
  ///
  /// In it, this message translates to:
  /// **'Altre azioni'**
  String get moreActions;

  /// No description provided for @renameServer.
  ///
  /// In it, this message translates to:
  /// **'Rinomina server'**
  String get renameServer;

  /// No description provided for @deleteServer.
  ///
  /// In it, this message translates to:
  /// **'Elimina server'**
  String get deleteServer;

  /// No description provided for @serverCardLine.
  ///
  /// In it, this message translates to:
  /// **'{version}  ·  :{port}  ·  {online}/{max} giocatori'**
  String serverCardLine(String version, int port, int online, int max);

  /// No description provided for @serverInfoLine.
  ///
  /// In it, this message translates to:
  /// **'Versione {version}  ·  Java {java}  ·  RAM {ramMin} / {ramMax}  ·  Porta {port}'**
  String serverInfoLine(
    String version,
    String java,
    String ramMin,
    String ramMax,
    int port,
  );

  /// No description provided for @deleteServerTitle.
  ///
  /// In it, this message translates to:
  /// **'Eliminare {name}?'**
  String deleteServerTitle(String name);

  /// No description provided for @deleteServersTitle.
  ///
  /// In it, this message translates to:
  /// **'Eliminare {count} server?'**
  String deleteServersTitle(int count);

  /// No description provided for @deleteServerMessage.
  ///
  /// In it, this message translates to:
  /// **'Il server viene rimosso dall\'elenco di VoxelPanel. Scegli se eliminare anche i file.'**
  String get deleteServerMessage;

  /// No description provided for @deleteServerFiles.
  ///
  /// In it, this message translates to:
  /// **'Elimina anche la cartella del server'**
  String get deleteServerFiles;

  /// No description provided for @deleteServerFilesHint.
  ///
  /// In it, this message translates to:
  /// **'Mondi, plugin e configurazioni verranno cancellati definitivamente.'**
  String get deleteServerFilesHint;

  /// No description provided for @deleteServerBackups.
  ///
  /// In it, this message translates to:
  /// **'Elimina anche i backup'**
  String get deleteServerBackups;

  /// No description provided for @tabOverview.
  ///
  /// In it, this message translates to:
  /// **'Panoramica'**
  String get tabOverview;

  /// No description provided for @tabConsole.
  ///
  /// In it, this message translates to:
  /// **'Console'**
  String get tabConsole;

  /// No description provided for @tabProperties.
  ///
  /// In it, this message translates to:
  /// **'Proprietà'**
  String get tabProperties;

  /// No description provided for @tabPlugins.
  ///
  /// In it, this message translates to:
  /// **'Plugin'**
  String get tabPlugins;

  /// No description provided for @tabWorlds.
  ///
  /// In it, this message translates to:
  /// **'Mondi'**
  String get tabWorlds;

  /// No description provided for @tabBackups.
  ///
  /// In it, this message translates to:
  /// **'Backup'**
  String get tabBackups;

  /// No description provided for @crashBanner.
  ///
  /// In it, this message translates to:
  /// **'Il server si è chiuso in modo inatteso (codice {code}). Controlla la console per i dettagli.'**
  String crashBanner(int code);

  /// No description provided for @eulaMissing.
  ///
  /// In it, this message translates to:
  /// **'L\'EULA di Minecraft non è stata accettata: il server non può avviarsi.'**
  String get eulaMissing;

  /// No description provided for @acceptEula.
  ///
  /// In it, this message translates to:
  /// **'Accetta l\'EULA'**
  String get acceptEula;

  /// No description provided for @statPlayers.
  ///
  /// In it, this message translates to:
  /// **'Giocatori'**
  String get statPlayers;

  /// No description provided for @noPlayersOnline.
  ///
  /// In it, this message translates to:
  /// **'Nessun giocatore online'**
  String get noPlayersOnline;

  /// No description provided for @statCpu.
  ///
  /// In it, this message translates to:
  /// **'CPU'**
  String get statCpu;

  /// No description provided for @statCpuHint.
  ///
  /// In it, this message translates to:
  /// **'Utilizzo del processo'**
  String get statCpuHint;

  /// No description provided for @statMemory.
  ///
  /// In it, this message translates to:
  /// **'Memoria'**
  String get statMemory;

  /// No description provided for @statMemoryHint.
  ///
  /// In it, this message translates to:
  /// **'Limite {max}'**
  String statMemoryHint(String max);

  /// No description provided for @statUptime.
  ///
  /// In it, this message translates to:
  /// **'Uptime'**
  String get statUptime;

  /// No description provided for @statUptimeHint.
  ///
  /// In it, this message translates to:
  /// **'Dall\'ultimo avvio'**
  String get statUptimeHint;

  /// No description provided for @liveStats.
  ///
  /// In it, this message translates to:
  /// **'Statistiche live'**
  String get liveStats;

  /// No description provided for @liveStatsSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Dati letti dal processo del server ogni 2 secondi.'**
  String get liveStatsSubtitle;

  /// No description provided for @serverOnline.
  ///
  /// In it, this message translates to:
  /// **'Server online'**
  String get serverOnline;

  /// No description provided for @serverOffline.
  ///
  /// In it, this message translates to:
  /// **'Server offline'**
  String get serverOffline;

  /// No description provided for @consoleSearch.
  ///
  /// In it, this message translates to:
  /// **'Cerca nella console'**
  String get consoleSearch;

  /// No description provided for @logInfo.
  ///
  /// In it, this message translates to:
  /// **'Info'**
  String get logInfo;

  /// No description provided for @logWarn.
  ///
  /// In it, this message translates to:
  /// **'Avvisi'**
  String get logWarn;

  /// No description provided for @logError.
  ///
  /// In it, this message translates to:
  /// **'Errori'**
  String get logError;

  /// No description provided for @consoleCopy.
  ///
  /// In it, this message translates to:
  /// **'Copia le righe visibili'**
  String get consoleCopy;

  /// No description provided for @consoleClear.
  ///
  /// In it, this message translates to:
  /// **'Pulisci la vista'**
  String get consoleClear;

  /// No description provided for @consoleScrollEnd.
  ///
  /// In it, this message translates to:
  /// **'Vai in fondo'**
  String get consoleScrollEnd;

  /// No description provided for @consoleWaiting.
  ///
  /// In it, this message translates to:
  /// **'In attesa di output...'**
  String get consoleWaiting;

  /// No description provided for @consoleEmpty.
  ///
  /// In it, this message translates to:
  /// **'Nessun output. Avvia il server per vedere la console.'**
  String get consoleEmpty;

  /// No description provided for @consoleCommandHint.
  ///
  /// In it, this message translates to:
  /// **'Comando (frecce su e giù per la cronologia)'**
  String get consoleCommandHint;

  /// No description provided for @consoleNotRunning.
  ///
  /// In it, this message translates to:
  /// **'Avvia il server per inviare comandi'**
  String get consoleNotRunning;

  /// No description provided for @propertiesSearch.
  ///
  /// In it, this message translates to:
  /// **'Cerca una proprietà'**
  String get propertiesSearch;

  /// No description provided for @propertiesMissing.
  ///
  /// In it, this message translates to:
  /// **'server.properties non esiste ancora: verrà creato al primo avvio.'**
  String get propertiesMissing;

  /// No description provided for @propertiesSaved.
  ///
  /// In it, this message translates to:
  /// **'Proprietà salvate. Riavvia il server per applicarle.'**
  String get propertiesSaved;

  /// No description provided for @propMotd.
  ///
  /// In it, this message translates to:
  /// **'MOTD'**
  String get propMotd;

  /// No description provided for @propPort.
  ///
  /// In it, this message translates to:
  /// **'Porta'**
  String get propPort;

  /// No description provided for @propMaxPlayers.
  ///
  /// In it, this message translates to:
  /// **'Giocatori massimi'**
  String get propMaxPlayers;

  /// No description provided for @propDifficulty.
  ///
  /// In it, this message translates to:
  /// **'Difficoltà'**
  String get propDifficulty;

  /// No description provided for @propGamemode.
  ///
  /// In it, this message translates to:
  /// **'Modalità di gioco'**
  String get propGamemode;

  /// No description provided for @propViewDistance.
  ///
  /// In it, this message translates to:
  /// **'Distanza di visualizzazione'**
  String get propViewDistance;

  /// No description provided for @propSimulationDistance.
  ///
  /// In it, this message translates to:
  /// **'Distanza di simulazione'**
  String get propSimulationDistance;

  /// No description provided for @propSpawnProtection.
  ///
  /// In it, this message translates to:
  /// **'Protezione dello spawn'**
  String get propSpawnProtection;

  /// No description provided for @propLevelName.
  ///
  /// In it, this message translates to:
  /// **'Nome del mondo'**
  String get propLevelName;

  /// No description provided for @propLevelSeed.
  ///
  /// In it, this message translates to:
  /// **'Seed'**
  String get propLevelSeed;

  /// No description provided for @propOnlineMode.
  ///
  /// In it, this message translates to:
  /// **'Online mode'**
  String get propOnlineMode;

  /// No description provided for @propWhitelist.
  ///
  /// In it, this message translates to:
  /// **'Whitelist'**
  String get propWhitelist;

  /// No description provided for @propPvp.
  ///
  /// In it, this message translates to:
  /// **'PvP'**
  String get propPvp;

  /// No description provided for @difficultyPeaceful.
  ///
  /// In it, this message translates to:
  /// **'Pacifica'**
  String get difficultyPeaceful;

  /// No description provided for @difficultyEasy.
  ///
  /// In it, this message translates to:
  /// **'Facile'**
  String get difficultyEasy;

  /// No description provided for @difficultyNormal.
  ///
  /// In it, this message translates to:
  /// **'Normale'**
  String get difficultyNormal;

  /// No description provided for @difficultyHard.
  ///
  /// In it, this message translates to:
  /// **'Difficile'**
  String get difficultyHard;

  /// No description provided for @gamemodeSurvival.
  ///
  /// In it, this message translates to:
  /// **'Sopravvivenza'**
  String get gamemodeSurvival;

  /// No description provided for @gamemodeCreative.
  ///
  /// In it, this message translates to:
  /// **'Creativa'**
  String get gamemodeCreative;

  /// No description provided for @gamemodeAdventure.
  ///
  /// In it, this message translates to:
  /// **'Avventura'**
  String get gamemodeAdventure;

  /// No description provided for @gamemodeSpectator.
  ///
  /// In it, this message translates to:
  /// **'Spettatore'**
  String get gamemodeSpectator;

  /// No description provided for @installJar.
  ///
  /// In it, this message translates to:
  /// **'Installa jar'**
  String get installJar;

  /// No description provided for @searchModrinth.
  ///
  /// In it, this message translates to:
  /// **'Cerca su Modrinth'**
  String get searchModrinth;

  /// No description provided for @stopToEditPlugins.
  ///
  /// In it, this message translates to:
  /// **'Ferma il server prima di modificare i plugin.'**
  String get stopToEditPlugins;

  /// No description provided for @noPlugins.
  ///
  /// In it, this message translates to:
  /// **'Nessun plugin installato.'**
  String get noPlugins;

  /// No description provided for @deletePluginTitle.
  ///
  /// In it, this message translates to:
  /// **'Eliminare il plugin?'**
  String get deletePluginTitle;

  /// No description provided for @pluginJarTitle.
  ///
  /// In it, this message translates to:
  /// **'Jar del plugin'**
  String get pluginJarTitle;

  /// No description provided for @searchPlugins.
  ///
  /// In it, this message translates to:
  /// **'Cerca plugin'**
  String get searchPlugins;

  /// No description provided for @downloadsCount.
  ///
  /// In it, this message translates to:
  /// **'{count} download'**
  String downloadsCount(int count);

  /// No description provided for @noWorlds.
  ///
  /// In it, this message translates to:
  /// **'Nessun mondo. Verrà creato al primo avvio.'**
  String get noWorlds;

  /// No description provided for @worldActive.
  ///
  /// In it, this message translates to:
  /// **'{name} (attivo)'**
  String worldActive(String name);

  /// No description provided for @setActiveWorld.
  ///
  /// In it, this message translates to:
  /// **'Imposta come mondo attivo'**
  String get setActiveWorld;

  /// No description provided for @restartToApply.
  ///
  /// In it, this message translates to:
  /// **'Fatto. Riavvia il server per applicare la modifica.'**
  String get restartToApply;

  /// No description provided for @deleteWorldTitle.
  ///
  /// In it, this message translates to:
  /// **'Eliminare il mondo {name}?'**
  String deleteWorldTitle(String name);

  /// No description provided for @deleteWorldMessage.
  ///
  /// In it, this message translates to:
  /// **'La cartella del mondo verrà cancellata definitivamente. Crea prima un backup se vuoi conservarla.'**
  String get deleteWorldMessage;

  /// No description provided for @createBackup.
  ///
  /// In it, this message translates to:
  /// **'Crea backup'**
  String get createBackup;

  /// No description provided for @backupRunningHint.
  ///
  /// In it, this message translates to:
  /// **'Un backup a server fermo è più coerente.'**
  String get backupRunningHint;

  /// No description provided for @noBackups.
  ///
  /// In it, this message translates to:
  /// **'Nessun backup.'**
  String get noBackups;

  /// No description provided for @restoreBackupTitle.
  ///
  /// In it, this message translates to:
  /// **'Ripristinare il backup?'**
  String get restoreBackupTitle;

  /// No description provided for @restoreBackupMessage.
  ///
  /// In it, this message translates to:
  /// **'I file attuali verranno sovrascritti. Prima del ripristino viene creata automaticamente una copia di sicurezza.'**
  String get restoreBackupMessage;

  /// No description provided for @deleteBackupTitle.
  ///
  /// In it, this message translates to:
  /// **'Eliminare il backup?'**
  String get deleteBackupTitle;

  /// No description provided for @createSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Crea un nuovo server o importa una cartella già esistente.'**
  String get createSubtitle;

  /// No description provided for @createMode.
  ///
  /// In it, this message translates to:
  /// **'Crea server'**
  String get createMode;

  /// No description provided for @createModeSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Configura e avvia un nuovo server'**
  String get createModeSubtitle;

  /// No description provided for @importModeSubtitle.
  ///
  /// In it, this message translates to:
  /// **'Importa da una cartella esistente'**
  String get importModeSubtitle;

  /// No description provided for @serverFolder.
  ///
  /// In it, this message translates to:
  /// **'Cartella del server'**
  String get serverFolder;

  /// No description provided for @serverJarTitle.
  ///
  /// In it, this message translates to:
  /// **'Jar del server'**
  String get serverJarTitle;

  /// No description provided for @importDetectedPaper.
  ///
  /// In it, this message translates to:
  /// **'Paper: {value}'**
  String importDetectedPaper(String value);

  /// No description provided for @importDetectedJava.
  ///
  /// In it, this message translates to:
  /// **'Java: {value}'**
  String importDetectedJava(String value);

  /// No description provided for @importDetectedRam.
  ///
  /// In it, this message translates to:
  /// **'RAM: {min} / {max}'**
  String importDetectedRam(String min, String max);

  /// No description provided for @importDetectedContent.
  ///
  /// In it, this message translates to:
  /// **'Plugin: {plugins} · Mondi: {worlds}'**
  String importDetectedContent(int plugins, int worlds);

  /// No description provided for @eulaCheckbox.
  ///
  /// In it, this message translates to:
  /// **'Accetto l\'EULA di Minecraft (eula=true)'**
  String get eulaCheckbox;

  /// No description provided for @eulaHint.
  ///
  /// In it, this message translates to:
  /// **'È necessario accettare l\'EULA di Minecraft per creare il server.'**
  String get eulaHint;

  /// No description provided for @serverName.
  ///
  /// In it, this message translates to:
  /// **'Nome del server'**
  String get serverName;

  /// No description provided for @serverNameHint.
  ///
  /// In it, this message translates to:
  /// **'Es. Il mio server'**
  String get serverNameHint;

  /// No description provided for @destinationFolder.
  ///
  /// In it, this message translates to:
  /// **'Cartella di destinazione'**
  String get destinationFolder;

  /// No description provided for @destinationFolderHint.
  ///
  /// In it, this message translates to:
  /// **'Vuota: VoxelPanel ne crea una nei dati locali'**
  String get destinationFolderHint;

  /// No description provided for @installMethod.
  ///
  /// In it, this message translates to:
  /// **'Metodo di installazione'**
  String get installMethod;

  /// No description provided for @methodAutomatic.
  ///
  /// In it, this message translates to:
  /// **'Automatica'**
  String get methodAutomatic;

  /// No description provided for @methodManual.
  ///
  /// In it, this message translates to:
  /// **'Manuale'**
  String get methodManual;

  /// No description provided for @memoryRam.
  ///
  /// In it, this message translates to:
  /// **'Memoria RAM'**
  String get memoryRam;

  /// No description provided for @paperVersion.
  ///
  /// In it, this message translates to:
  /// **'Versione Paper'**
  String get paperVersion;

  /// No description provided for @latestVersion.
  ///
  /// In it, this message translates to:
  /// **'{version} (ultima)'**
  String latestVersion(String version);

  /// No description provided for @installedJava.
  ///
  /// In it, this message translates to:
  /// **'Runtime Java installato'**
  String get installedJava;

  /// No description provided for @downloadOtherJava.
  ///
  /// In it, this message translates to:
  /// **'Scarica un altro Java'**
  String get downloadOtherJava;

  /// No description provided for @noLocalJar.
  ///
  /// In it, this message translates to:
  /// **'Nessun jar locale'**
  String get noLocalJar;

  /// No description provided for @chooseJar.
  ///
  /// In it, this message translates to:
  /// **'Scegli jar'**
  String get chooseJar;

  /// No description provided for @ramMin.
  ///
  /// In it, this message translates to:
  /// **'RAM minima'**
  String get ramMin;

  /// No description provided for @ramMax.
  ///
  /// In it, this message translates to:
  /// **'RAM massima'**
  String get ramMax;

  /// No description provided for @jvmFlags.
  ///
  /// In it, this message translates to:
  /// **'Flag JVM'**
  String get jvmFlags;

  /// No description provided for @javaVersion.
  ///
  /// In it, this message translates to:
  /// **'Versione Java'**
  String get javaVersion;

  /// No description provided for @installing.
  ///
  /// In it, this message translates to:
  /// **'Installazione...'**
  String get installing;

  /// No description provided for @createServer.
  ///
  /// In it, this message translates to:
  /// **'Crea server'**
  String get createServer;

  /// No description provided for @issueMissingName.
  ///
  /// In it, this message translates to:
  /// **'Inserisci un nome.'**
  String get issueMissingName;

  /// No description provided for @issueEula.
  ///
  /// In it, this message translates to:
  /// **'Accetta l\'EULA di Minecraft per continuare.'**
  String get issueEula;

  /// No description provided for @issueMissingVersion.
  ///
  /// In it, this message translates to:
  /// **'Seleziona una versione.'**
  String get issueMissingVersion;

  /// No description provided for @issueMissingVersionOrJar.
  ///
  /// In it, this message translates to:
  /// **'Seleziona una versione oppure un jar.'**
  String get issueMissingVersionOrJar;

  /// No description provided for @issueMissingJava.
  ///
  /// In it, this message translates to:
  /// **'Seleziona un runtime Java.'**
  String get issueMissingJava;

  /// No description provided for @errorServerRunning.
  ///
  /// In it, this message translates to:
  /// **'Ferma il server prima di continuare.'**
  String get errorServerRunning;

  /// No description provided for @errorServerStopped.
  ///
  /// In it, this message translates to:
  /// **'Il server non è in esecuzione.'**
  String get errorServerStopped;

  /// No description provided for @errorEulaRequired.
  ///
  /// In it, this message translates to:
  /// **'Accetta l\'EULA di Minecraft prima di avviare il server.'**
  String get errorEulaRequired;

  /// No description provided for @errorNetwork.
  ///
  /// In it, this message translates to:
  /// **'Errore di rete: {details}'**
  String errorNetwork(String details);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'it'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'it':
      return AppLocalizationsIt();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
