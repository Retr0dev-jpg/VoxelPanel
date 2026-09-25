import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:voxel_panel/screens/server/console_tab.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/create_wizard_body.dart';
import 'package:voxel_panel/widgets/server_list_view.dart';

import 'helpers.dart';

const _survival = ServerSummary(
  id: 'abc',
  name: 'Survival',
  root: '/srv/survival',
  paperVersion: '1.21.1',
  javaMajor: 21,
  ramMin: '2G',
  ramMax: '4G',
  port: 25565,
  maxPlayers: 20,
);

CreateInput _input({String name = 'Survival', bool eula = true, bool automatic = true, String version = '1.21.1', String javaHome = ''}) {
  return CreateInput(name: name, acceptEula: eula, automatic: automatic, paperVersion: version, jarPath: '', javaHome: javaHome, javaMajor: 0);
}

void main() {
  test('la creazione richiede nome, EULA e versione', () {
    expect(validateCreate(_input(name: '')), CreateIssue.missingName);
    expect(validateCreate(_input(eula: false)), CreateIssue.eulaNotAccepted);
    expect(validateCreate(_input(automatic: false, version: '', javaHome: '/jdk')), CreateIssue.missingVersionOrJar);
    expect(validateCreate(_input(automatic: false)), CreateIssue.missingJava);
    expect(validateCreate(_input()), isNull);
  });

  test('il buffer della console scarta le righe più vecchie', () {
    final buffer = LineBuffer(3);
    for (var i = 0; i < 5; i++) {
      buffer.add('riga $i');
    }
    expect(buffer.lines.toList(), ['riga 2', 'riga 3', 'riga 4']);
  });

  test('la cronologia dei comandi scorre come una shell', () {
    final history = CommandHistory()
      ..push('list')
      ..push('say ciao');
    expect(history.previous(), 'say ciao');
    expect(history.previous(), 'list');
    expect(history.previous(), 'list');
    expect(history.next(), 'say ciao');
    expect(history.next(), '');
  });

  test('i livelli di log vengono riconosciuti', () {
    expect(levelOf('[12:00:00 WARN]: Can\'t keep up!'), LogLevel.warn);
    expect(levelOf('[12:00:00 ERROR]: boom'), LogLevel.error);
    expect(levelOf('[12:00:00 INFO]: Done'), LogLevel.info);
  });

  testWidgets('la lista mostra il server e il pulsante Avvia', (tester) async {
    await pumpApp(tester, ServerListView(servers: const [_survival], onOpen: (_) {}, onStart: (_) {}, onStop: (_) {}, onRestart: (_) {}));
    expect(find.text('Survival'), findsOneWidget);
    expect(find.text('Avvia'), findsOneWidget);
    expect(find.textContaining('25565'), findsOneWidget);
  });

  testWidgets('un server online mostra giocatori e pulsanti di arresto', (tester) async {
    const runtime = ServerRuntime(
      serverId: 'abc',
      status: ServerStatus.running,
      pid: 42,
      startedUnix: 0,
      players: ['Steve'],
      cpuPercent: 10,
      memoryBytes: 1024,
      crashed: false,
    );
    await pumpApp(
      tester,
      ServerListView(servers: const [_survival], onOpen: (_) {}, onStart: (_) {}, onStop: (_) {}, onRestart: (_) {}),
      runtime: const RuntimeState(servers: {'abc': runtime}),
    );
    expect(find.text('Ferma'), findsOneWidget);
    expect(find.text('Riavvia'), findsOneWidget);
    expect(find.textContaining('1/20'), findsOneWidget);
  });

  testWidgets('il wizard blocca la creazione senza nome', (tester) async {
    await pumpApp(
      tester,
      ListView(
        children: [
          CreateWizardBody(
            paperVersions: const ['1.21.1'],
            ramChoices: const [RamChoice(megabytes: 2048, label: '2 GB', value: '2G')],
            jvmFlags: const [JvmFlagChoice(flag: '-XX:+UseG1GC', recommended: true)],
            runtimes: const [],
            javaReleases: const [],
            suggestedMin: '2G',
            suggestedMax: '4G',
            progress: const [],
            busy: false,
            onAuto: (_) async {},
            onManual: (_) async {},
            pickDirectory: () async => null,
            pickJar: () async => null,
            installJava: (_) async {},
          ),
        ],
      ),
    );
    final create = find.widgetWithText(FilledButton, 'Crea server');
    await tester.ensureVisible(create);
    await tester.pumpAndSettle();
    await tester.tap(create);
    await tester.pump();
    expect(find.text('Inserisci un nome.'), findsOneWidget);
  });
}
