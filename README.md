# VoxelPanel

App desktop per Windows che crea, importa e gestisce più server [Paper](https://papermc.io/) in locale.

L’interfaccia è in Flutter (Material 3, italiano e inglese). Il motore è in Rust e viene chiamato da Flutter con [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge). Chiudere l’app ferma i server che ha avviato.

## Cosa fa

- Crea un server nuovo (procedura automatica o manuale) oppure importa una cartella già esistente, senza copiare i mondi.
- Installa Java da Adoptium, scarica Paper dall’API Fill v3 e scrive EULA, RAM, flag JVM e `start.bat`.
- Avvia `java` come processo figlio, con console, stop, riavvio e arresto di tutti i server alla chiusura.
- Mostra lo stato reale di ogni server: "Avvio" finché il log non segnala la fine del caricamento, poi "Online"; rileva i crash con il codice di uscita.
- Statistiche live di CPU e RAM, uptime ed elenco dei giocatori collegati, inviati dal motore Rust senza polling.
- Console con cronologia dei comandi (frecce su e giù), ricerca, filtro per livello, copia e limite di righe.
- Modifica `server.properties`, gestisce plugin locali e da Modrinth, mondi, backup e ripristino.
- Rinomina ed elimina i server (anche più server insieme), con scelta se cancellare file e backup.

La rete serve solo per installare o aggiornare Java, Paper o un plugin. Stato, console e file restano sul disco.

## Dati locali

L’indice sta in `%LOCALAPPDATA%\VoxelPanel\catalog.json`. I JDK condivisi stanno in `runtimes\`. Ogni server è una cartella scelta dall’utente (di default `servers\<id>\`) con `server.json`, jar Paper, `eula.txt`, `server.properties`, `plugins\`, mondi e `logs\`.

I backup stanno in `backups\`, fuori dalla cartella del server.

## Struttura

- `rust/src/api/`: funzioni esposte a Flutter. Gli errori sono `PanelError` (codice più messaggio), le operazioni lunghe inviano `ProgressEvent` in tempo reale, `watch_events` trasmette lo stato di runtime di ogni server.
- `rust/src/process.rs`: supervisor dei processi (stato, giocatori, uscita, campionamento CPU e RAM).
- `lib/src/providers.dart`: stato Riverpod (lista dei server, dettagli, runtime).
- `lib/screens/server/`: una tab per file (panoramica, console, proprietà, plugin, mondi, backup).
- `lib/widgets/common/`: widget condivisi.
- `lib/l10n/`: testi dell’interfaccia (`app_it.arb` è il modello, `app_en.arb` la traduzione).

## Sviluppo

```bash
flutter pub get
flutter run -d windows
```

Dopo ogni modifica alle funzioni in `rust/src/api/` vanno rigenerati i binding:

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0
flutter_rust_bridge_codegen generate
```

Controlli eseguiti anche dalla CI su ogni pull request:

```bash
cd rust && cargo clippy --all-targets -- -D warnings && cargo test
flutter analyze
flutter test
```
