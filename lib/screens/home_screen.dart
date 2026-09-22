import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/create_server_screen.dart';
import 'package:voxel_panel/screens/server_screen.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/server_list_view.dart';

final serverListProvider = AsyncNotifierProvider<ServerListNotifier, List<ServerSummary>>(
  ServerListNotifier.new,
);

class ServerListNotifier extends AsyncNotifier<List<ServerSummary>> {
  @override
  Future<List<ServerSummary>> build() => listServers();

  Future<void> reload() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(listServers);
  }
}

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final servers = ref.watch(serverListProvider);
    return Scaffold(
      appBar: AppBar(
        title: const Text('VoxelPanel'),
        actions: [
          IconButton(
            tooltip: 'Aggiorna',
            onPressed: () => ref.read(serverListProvider.notifier).reload(),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: servers.when(
        data: (items) => ServerListView(
          servers: items,
          onOpen: (server) => _open(context, ref, server.id),
          onStart: (server) => _run(context, ref, () => startServer(id: server.id)),
          onStop: (server) => _run(context, ref, () => stopServer(id: server.id)),
        ),
        error: (error, stack) => Center(child: Text(readableError(error))),
        loading: () => const Center(child: CircularProgressIndicator()),
      ),
      floatingActionButton: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          FloatingActionButton.extended(
            heroTag: 'import',
            onPressed: () => _import(context, ref),
            icon: const Icon(Icons.drive_folder_upload),
            label: const Text('Importa'),
          ),
          const SizedBox(height: 12),
          FloatingActionButton.extended(
            heroTag: 'create',
            onPressed: () async {
              final created = await Navigator.push<bool>(
                context,
                MaterialPageRoute(builder: (context) => const CreateServerScreen()),
              );
              if (created == true) {
                await ref.read(serverListProvider.notifier).reload();
              }
            },
            icon: const Icon(Icons.add),
            label: const Text('Nuovo server'),
          ),
        ],
      ),
    );
  }

  Future<void> _open(BuildContext context, WidgetRef ref, String id) async {
    await Navigator.push(context, MaterialPageRoute(builder: (context) => ServerScreen(serverId: id)));
    await ref.read(serverListProvider.notifier).reload();
  }

  Future<void> _run(BuildContext context, WidgetRef ref, Future<void> Function() action) async {
    try {
      await action();
      await ref.read(serverListProvider.notifier).reload();
    } catch (error) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }

  Future<void> _import(BuildContext context, WidgetRef ref) async {
    final path = await FilePicker.getDirectoryPath(dialogTitle: 'Cartella del server');
    if (path == null || !context.mounted) {
      return;
    }
    try {
      final preview = await previewImport(path: path);
      if (!context.mounted) {
        return;
      }
      var acceptEula = preview.hasEula;
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (context) => StatefulBuilder(
          builder: (context, setState) => AlertDialog(
            title: const Text('Importa cartella'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(preview.root),
                const SizedBox(height: 8),
                Text('Paper: ${preview.paperVersion.isEmpty ? 'non rilevato' : preview.paperVersion}'),
                Text('Java: ${preview.javaMajor == 0 ? 'non rilevato' : preview.javaMajor}'),
                Text('RAM: ${preview.ramMin} / ${preview.ramMax}'),
                Text('Plugin: ${preview.pluginCount} · Mondi: ${preview.worldCount}'),
                CheckboxListTile(
                  contentPadding: EdgeInsets.zero,
                  value: acceptEula,
                  onChanged: (value) => setState(() => acceptEula = value ?? false),
                  title: const Text("Accetto l'EULA di Minecraft"),
                ),
              ],
            ),
            actions: [
              TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Annulla')),
              FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Importa')),
            ],
          ),
        ),
      );
      if (confirmed != true) {
        return;
      }
      await importServer(path: path, name: preview.name, acceptEula: acceptEula);
      await ref.read(serverListProvider.notifier).reload();
    } catch (error) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }
}
