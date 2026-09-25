# VoxelPanel

App desktop per Windows, Linux e macOS che crea, importa e gestisce più server Minecraft in locale: Vanilla, server a plugin, proxy e altro.

L’interfaccia è in Flutter (Material 3, italiano e inglese). Il motore è in Rust e viene chiamato da Flutter con [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge). Chiudere l’app ferma i server che ha avviato.

## Cosa fa

- Crea un server con una procedura a passi (software, versione e build, Java e memoria, impostazioni iniziali, riepilogo) oppure importa una cartella già esistente riconoscendo il software, senza copiare i mondi.
- Installa Java da Adoptium (x64 o ARM64, per il sistema in uso) nella versione richiesta dal server, scarica il software verificandone il checksum e scrive EULA, `server.properties`, RAM, flag JVM e lo script di avvio (`start.bat` su Windows, `start.sh` su Linux e macOS).
- Cambia versione o build di un server esistente, con backup automatico prima dell’aggiornamento.
- Avvia `java` come processo figlio, con console, stop, riavvio e arresto di tutti i server alla chiusura.
- Mostra lo stato reale di ogni server: "Avvio" finché il log non segnala la fine del caricamento, poi "Online"; rileva i crash con il codice di uscita.
- Statistiche live di CPU e RAM, uptime ed elenco dei giocatori collegati, inviati dal motore Rust senza polling.
- Console con cronologia dei comandi (frecce su e giù), ricerca, filtro per livello, copia e limite di righe.
- Modifica `server.properties`, gestisce plugin locali e da Modrinth, mondi, backup e ripristino.
- Rinomina ed elimina i server (anche più server insieme), con scelta se cancellare file e backup.
- Impostazioni del launcher: lingua, tema chiaro o scuro con colore di accento, avvio con il sistema, chiusura nel tray, cartelle di server, backup, runtime e cache (spostabili con migrazione guidata), runtime Java installati e di sistema con versione preferita, valori predefiniti per i nuovi server (RAM, preset JVM Aikar/G1/ZGC, porta), console, backup (conservazione, compressione, esclusioni), proxy e timeout di rete, chiave CurseForge, notifiche desktop, log dell'app, esportazione e importazione.

La rete serve solo per installare o aggiornare Java, Paper o un plugin. Stato, console e file restano sul disco.

## Software supportato

| Categoria | Software | Fonte |
| --- | --- | --- |
| Vanilla | Vanilla (anche snapshot) | Manifest Mojang |
| Plugin | Paper, Folia | API Fill v3 di PaperMC |
| Plugin | Purpur | api.purpurmc.org |
| Plugin | Pufferfish | Jenkins di Pufferfish |
| Plugin | Leaf | api.leafmc.one |
| Plugin | Spigot | Compilato in locale con BuildTools (richiede Git) |
| Proxy | Velocity | API Fill v3 di PaperMC |
| Altro | Jar personalizzato | File scelto dall’utente |

Gli elenchi delle versioni vengono salvati nella cartella della cache: se la rete non risponde il wizard usa l’ultima copia scaricata.

## Piattaforme

| Sistema | Cartella dati | Pacchetto di release |
| --- | --- | --- |
| Windows | `%LOCALAPPDATA%\VoxelPanel` | `VoxelPanel-windows-x64.zip` |
| Linux | `~/.local/share/VoxelPanel` | `VoxelPanel-linux-x64.tar.gz` |
| macOS | `~/Library/Application Support/VoxelPanel` | `VoxelPanel-macos.dmg` |

La variabile d’ambiente `VOXELPANEL_DATA_DIR` sostituisce la cartella dati (utile per test o installazioni portabili).

I server avviati vengono chiusi insieme a VoxelPanel anche in caso di crash dell’app: su Windows tramite un job object, su Linux con il segnale di morte del processo padre. Su Linux e macOS ogni server ha il proprio gruppo di processi, così l’arresto forzato raggiunge anche i processi figli.

Su Linux la scelta delle cartelle usa `zenity` o `kdialog`. L’app per macOS non è firmata: al primo avvio va aperta con clic destro e poi "Apri".

## Dati locali

Nella cartella dati ci sono `settings.json` (impostazioni del launcher, con numero di schema e migrazioni automatiche) e `logs/` (log giornalieri dell’app, conservati 7 giorni). L’indice dei server sta in `catalog.json` e i JDK condivisi in `runtimes/`. Ogni server è una cartella scelta dall’utente (di default `servers/<id>/`) con `server.json` (schema 2: software, versione, build e comando di avvio; i file dello schema 1 vengono migrati automaticamente), il jar del server, `eula.txt`, `server.properties`, `plugins/`, mondi e `logs/`.

I backup stanno in `backups/`, fuori dalla cartella del server.

## Struttura

- `rust/src/api/`: funzioni esposte a Flutter. Gli errori sono `PanelError` (codice più messaggio), le operazioni lunghe inviano `ProgressEvent` in tempo reale, `watch_events` trasmette lo stato di runtime di ogni server.
- `rust/src/providers/`: un modulo per fonte di software (trait `Provider`: versioni, build, Java richiesto, installazione) e rilevamento del software all’import.
- `rust/src/cache.rs`: cache su disco delle risposte delle API con scadenza e uso offline.
- `rust/src/process.rs`: supervisor dei processi (stato, giocatori, uscita, campionamento CPU e RAM).
- `rust/src/platform/`: codice specifico per sistema operativo (eseguibile Java, gruppi di processi e job object, arresto forzato, script di avvio, apertura cartelle).
- `rust/src/launcher_settings.rs`: lettura, migrazione, validazione e salvataggio delle impostazioni; spostamento delle cartelle gestite.
- `lib/src/providers.dart`, `lib/src/settings.dart`: stato Riverpod (server, runtime, impostazioni).
- `lib/src/desktop_integration.dart`: chiusura della finestra, tray, notifiche e controllo aggiornamenti.
- `lib/screens/settings/`: schermata delle impostazioni divisa per sezioni.
- `lib/screens/create/`: wizard di creazione a passi e relativa validazione.
- `lib/screens/server/`: una tab per file (panoramica, console, proprietà, plugin, mondi, backup).
- `lib/widgets/common/`: widget condivisi.
- `lib/l10n/`: testi dell’interfaccia (`app_it.arb` è il modello, `app_en.arb` la traduzione).

## Sviluppo

```bash
flutter pub get
flutter run -d windows   # oppure linux, macos
```

Su Linux servono `clang cmake ninja-build pkg-config libgtk-3-dev libnotify-dev libayatana-appindicator3-dev`.

Dopo ogni modifica alle funzioni in `rust/src/api/` vanno rigenerati i binding:

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0
flutter_rust_bridge_codegen generate
```

I test che contattano le API reali dei provider sono esclusi di default:

```bash
cd rust && cargo test live_ -- --ignored
```

Controlli eseguiti anche dalla CI su ogni pull request (i test Rust girano su Windows, Linux e macOS):

```bash
cd rust && cargo clippy --all-targets -- -D warnings && cargo test
flutter analyze
flutter test
```

## Release

Ogni push su `main` compila l’app per i tre sistemi. La release `v<versione>` viene pubblicata solo se non esiste già: per rilasciare basta aumentare `version` in `pubspec.yaml`.
