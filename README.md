# VoxelPanel

App desktop per Windows che crea, importa e gestisce più server [Paper](https://papermc.io/) in locale.

L’interfaccia è in Flutter (Material 3, testi in italiano). Il motore è in Rust e viene chiamato da Flutter con [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge). Chiudere l’app ferma i server che ha avviato.

## Cosa fa

- Crea un server nuovo (procedura automatica o manuale) oppure importa una cartella già esistente, senza copiare i mondi.
- Installa Java da Adoptium, scarica Paper dall’API Fill v3 e scrive EULA, RAM, flag JVM e `start.bat`.
- Avvia `java` come processo figlio, con console, stop, riavvio e arresto di tutti i server alla chiusura.
- Modifica `server.properties`, gestisce plugin locali e da Modrinth, mondi, backup e ripristino.

La rete serve solo per installare o aggiornare Java, Paper o un plugin. Stato, console e file restano sul disco.

## Dati locali

L’indice sta in `%LOCALAPPDATA%\VoxelPanel\catalog.json`. I JDK condivisi stanno in `runtimes\`. Ogni server è una cartella scelta dall’utente (di default `servers\<id>\`) con `server.json`, jar Paper, `eula.txt`, `server.properties`, `plugins\`, mondi e `logs\`.

I backup stanno in `backups\`, fuori dalla cartella del server.

## Avvio

```bash
flutter run -d windows
```
