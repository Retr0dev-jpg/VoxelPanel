import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:voxel_panel/main.dart';
import 'package:voxel_panel/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('mostra VoxelPanel', (tester) async {
    await tester.pumpWidget(const ProviderScope(child: VoxelApp()));
    await tester.pump();
    expect(find.text('VoxelPanel'), findsWidgets);
  });
}
