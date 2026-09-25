// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/create_server_screen.dart';
import 'package:voxel_panel/screens/server_screen.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
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

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  var _query = '';

  @override
  Widget build(BuildContext context) {
    final servers = ref.watch(serverListProvider);
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            entries: const [SidebarEntry(label: 'Server', icon: Icons.dns_outlined)],
            selected: 0,
            onSelected: (_) {},
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(28, 24, 28, 8),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: [
                      const Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('Server', style: TextStyle(fontSize: 32, fontWeight: FontWeight.w700)),
                            SizedBox(height: 4),
                            Text('Crea, gestisci e avvia i tuoi server Paper in locale.', style: TextStyle(color: panelMuted)),
                          ],
                        ),
                      ),
                      SizedBox(
                        width: 260,
                        child: TextField(
                          decoration: const InputDecoration(prefixIcon: Icon(Icons.search), hintText: 'Cerca server...'),
                          onChanged: (value) => setState(() => _query = value.trim().toLowerCase()),
                        ),
                      ),
                      const SizedBox(width: 12),
                      FilledButton.icon(
                        onPressed: _create,
                        style: FilledButton.styleFrom(minimumSize: const Size(0, 56), padding: const EdgeInsets.symmetric(horizontal: 18)),
                        icon: const Icon(Icons.add),
                        label: const Text('Nuovo server'),
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: servers.when(
                    data: (items) {
                      final visible = items.where((server) {
                        if (_query.isEmpty) {
                          return true;
                        }
                        return server.name.toLowerCase().contains(_query) || '${server.port}'.contains(_query);
                      }).toList();
                      return ServerListView(
                        servers: visible,
                        onOpen: (server) => _open(context, ref, server.id),
                        onStart: (server) => _run(context, ref, () => startServer(id: server.id)),
                        onStop: (server) => _run(context, ref, () => stopServer(id: server.id)),
                        onRestart: (server) => _run(context, ref, () => restartServer(id: server.id)),
                      );
                    },
                    error: (error, stack) => Center(child: Text(readableError(error))),
                    loading: () => const Center(child: CircularProgressIndicator()),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _create({bool importing = false}) async {
    final created = await Navigator.push<bool>(
      context,
      MaterialPageRoute(builder: (context) => CreateServerScreen(startOnImport: importing)),
    );
    if (created == true) {
      await ref.read(serverListProvider.notifier).reload();
    }
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

}
