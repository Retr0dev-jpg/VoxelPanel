import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/theme.dart';

/// Runtime state without the Rust event stream, for widget tests.
class FakeRuntimeNotifier extends RuntimeNotifier {
  FakeRuntimeNotifier([this.initial = const RuntimeState()]);

  final RuntimeState initial;

  @override
  RuntimeState build() => initial;
}

Future<void> pumpApp(WidgetTester tester, Widget child, {RuntimeState runtime = const RuntimeState()}) {
  return tester.pumpWidget(
    ProviderScope(
      overrides: [runtimeProvider.overrideWith(() => FakeRuntimeNotifier(runtime))],
      child: MaterialApp(
        theme: voxelTheme(),
        locale: const Locale('it'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(body: child),
      ),
    ),
  );
}
