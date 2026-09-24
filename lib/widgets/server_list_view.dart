// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

class ServerListView extends StatelessWidget {
  const ServerListView({
    super.key,
    required this.servers,
    required this.onOpen,
    required this.onStart,
    required this.onStop,
  });

  final List<ServerSummary> servers;
  final ValueChanged<ServerSummary> onOpen;
  final ValueChanged<ServerSummary> onStart;
  final ValueChanged<ServerSummary> onStop;

  @override
  Widget build(BuildContext context) {
    if (servers.isEmpty) {
      return const Center(
        child: Text('Nessun server. Creane uno o importa una cartella esistente.'),
      );
    }

    return ListView.separated(
      padding: const EdgeInsets.all(16),
      itemCount: servers.length,
      separatorBuilder: (context, index) => const SizedBox(height: 12),
      itemBuilder: (context, index) {
        final server = servers[index];
        final running = server.status == ServerStatus.running || server.status == ServerStatus.starting;
        return Card(
          child: ListTile(
            onTap: () => onOpen(server),
            title: Text(server.name),
            subtitle: Text(
              'Paper ${server.paperVersion ?? 'n/d'} · porta ${server.port} · ${server.ramMin} / ${server.ramMax}',
            ),
            trailing: Wrap(
              spacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Chip(label: Text(statusLabel(server.status))),
                if (running)
                  FilledButton.tonal(onPressed: () => onStop(server), child: const Text('Stop'))
                else if (server.status != ServerStatus.stopping)
                  FilledButton(onPressed: () => onStart(server), child: const Text('Avvia')),
              ],
            ),
          ),
        );
      },
    );
  }
}
