import 'package:flutter_test/flutter_test.dart';
import 'package:voxel_panel/screens/create/create_validation.dart';
import 'package:voxel_panel/screens/server/console_tab.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/provider_icon.dart';
import 'package:voxel_panel/widgets/server_list_view.dart';

import 'helpers.dart';

const _survival = ServerSummary(
  id: 'abc',
  name: 'Survival',
  root: '/srv/survival',
  provider: ProviderKind.purpur,
  mcVersion: '1.21.1',
  javaMajor: 21,
  ramMin: '2G',
  ramMax: '4G',
  port: 25565,
  maxPlayers: 20,
);

WizardInput _input({
  String name = 'Survival',
  bool provider = true,
  bool custom = false,
  String jar = '',
  String version = '1.21.1',
  int ramMin = 1024,
  int ramMax = 4096,
  int port = 0,
  int players = 20,
  bool eula = true,
  bool needsEula = true,
}) {
  return WizardInput(
    name: name,
    hasProvider: provider,
    isCustom: custom,
    customJar: jar,
    version: version,
    ramMinMb: ramMin,
    ramMaxMb: ramMax,
    port: port,
    maxPlayers: players,
    acceptEula: eula,
    needsEula: needsEula,
  );
}

void main() {
  test('ogni passo del wizard valida i propri campi', () {
    expect(validateStep(WizardStep.software, _input(name: ' ')), CreateIssue.missingName);
    expect(validateStep(WizardStep.software, _input(provider: false)), CreateIssue.missingProvider);
    expect(validateStep(WizardStep.version, _input(version: '')), CreateIssue.missingVersion);
    expect(validateStep(WizardStep.version, _input(custom: true, version: '')), CreateIssue.missingJar);
    expect(validateStep(WizardStep.version, _input(custom: true, jar: '/srv/x.jar', version: '')), isNull);
    expect(validateStep(WizardStep.runtime, _input(ramMin: 8192, ramMax: 4096)), CreateIssue.invalidRam);
    expect(validateStep(WizardStep.settings, _input(port: 80)), CreateIssue.invalidPort);
    expect(validateStep(WizardStep.settings, _input(players: 0)), CreateIssue.invalidPlayers);
    expect(validateStep(WizardStep.summary, _input(eula: false)), CreateIssue.eulaNotAccepted);
    expect(validateStep(WizardStep.summary, _input(eula: false, needsEula: false)), isNull);
    expect(validateAll(_input()), isNull);
  });

  test('i valori di memoria si convertono in entrambi i sensi', () {
    expect(memoryValue(4096), '4G');
    expect(memoryValue(1536), '1536M');
    expect(parseMemoryMb('6G'), 6144);
    expect(parseMemoryMb('512m'), 512);
    expect(parseMemoryMb('tanta'), isNull);
  });

  test('gli id dei provider corrispondono ai file delle icone', () {
    expect(providerId(ProviderKind.neoForge), 'neoforge');
    expect(providerId(ProviderKind.spongeVanilla), 'spongevanilla');
    expect(providerId(ProviderKind.paper), 'paper');
  });

  test('il buffer della console scarta le righe più vecchie', () {
    final buffer = LineBuffer(3);
    for (var i = 0; i < 5; i++) {
      buffer.add('riga $i');
    }
    expect(buffer.lines.toList(), ['riga 2', 'riga 3', 'riga 4']);
    buffer.capacity = 2;
    expect(buffer.lines.toList(), ['riga 3', 'riga 4']);
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
    expect(find.textContaining('1.21.1'), findsOneWidget);
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

  testWidgets('un crash viene segnalato nella lista', (tester) async {
    const runtime = ServerRuntime(serverId: 'abc', status: ServerStatus.stopped, players: [], cpuPercent: 0, memoryBytes: 0, lastExitCode: 1, crashed: true);
    await pumpApp(
      tester,
      ServerListView(servers: const [_survival], onOpen: (_) {}, onStart: (_) {}, onStop: (_) {}, onRestart: (_) {}),
      runtime: const RuntimeState(servers: {'abc': runtime}),
    );
    expect(find.text('Crash'), findsOneWidget);
  });
}
