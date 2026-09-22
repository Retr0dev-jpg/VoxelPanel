import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/create_wizard_body.dart';
import 'package:voxel_panel/widgets/server_list_view.dart';

void main() {
  test('la creazione richiede nome, EULA e versione', () {
    expect(
      validateCreate(
        const CreateInput(
          name: '',
          acceptEula: true,
          automatic: true,
          paperVersion: '1.21.1',
          jarPath: '',
          javaHome: '',
          javaMajor: 0,
        ),
      ),
      'Inserisci un nome.',
    );
    expect(
      validateCreate(
        const CreateInput(
          name: 'Survival',
          acceptEula: false,
          automatic: true,
          paperVersion: '1.21.1',
          jarPath: '',
          javaHome: '',
          javaMajor: 0,
        ),
      ),
      "Accetta l'EULA di Minecraft per continuare.",
    );
    expect(
      validateCreate(
        const CreateInput(
          name: 'Survival',
          acceptEula: true,
          automatic: false,
          paperVersion: '',
          jarPath: '',
          javaHome: r'C:\runtime\jdk-21',
          javaMajor: 0,
        ),
      ),
      'Seleziona una versione Paper oppure un jar.',
    );
    expect(
      validateCreate(
        const CreateInput(
          name: 'Survival',
          acceptEula: true,
          automatic: true,
          paperVersion: '1.21.1',
          jarPath: '',
          javaHome: '',
          javaMajor: 0,
        ),
      ),
      isNull,
    );
  });

  testWidgets('la lista vuota invita a creare un server', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: ServerListView(servers: const [], onOpen: (_) {}, onStart: (_) {}, onStop: (_) {}),
      ),
    );
    expect(find.textContaining('Nessun server'), findsOneWidget);
  });

  testWidgets('la lista mostra il server e il pulsante Avvia', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: ServerListView(
          servers: const [
            ServerSummary(
              id: 'abc',
              name: 'Survival',
              root: r'D:\server',
              paperVersion: '1.21.1',
              javaMajor: 21,
              ramMin: '2G',
              ramMax: '4G',
              port: 25565,
              status: ServerStatus.stopped,
            ),
          ],
          onOpen: (_) {},
          onStart: (_) {},
          onStop: (_) {},
        ),
      ),
    );
    expect(find.text('Survival'), findsOneWidget);
    expect(find.text('Avvia'), findsOneWidget);
    expect(find.textContaining('25565'), findsOneWidget);
  });

  testWidgets('il wizard blocca la creazione senza EULA', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CreateWizardBody(
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
        ),
      ),
    );
    await tester.tap(find.text('Crea server'));
    await tester.pump();
    expect(find.text('Inserisci un nome.'), findsOneWidget);
  });
}
