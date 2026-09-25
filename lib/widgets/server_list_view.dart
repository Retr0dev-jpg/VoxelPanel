// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';

class ServerListView extends StatelessWidget {
  const ServerListView({
    super.key,
    required this.servers,
    required this.onOpen,
    required this.onStart,
    required this.onStop,
    this.onRestart,
  });

  final List<ServerSummary> servers;
  final ValueChanged<ServerSummary> onOpen;
  final ValueChanged<ServerSummary> onStart;
  final ValueChanged<ServerSummary> onStop;
  final ValueChanged<ServerSummary>? onRestart;

  @override
  Widget build(BuildContext context) {
    if (servers.isEmpty) {
      return const Center(child: Text('Nessun server. Creane uno o importa una cartella esistente.'));
    }

    return ListView.separated(
      padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
      itemCount: servers.length,
      separatorBuilder: (context, index) => const SizedBox(height: 12),
      itemBuilder: (context, index) {
        final server = servers[index];
        final running = server.status == ServerStatus.running || server.status == ServerStatus.starting;
        final online = server.status == ServerStatus.running;
        return Material(
          color: panelCard,
          borderRadius: BorderRadius.circular(16),
          child: InkWell(
            borderRadius: BorderRadius.circular(16),
            onTap: () => onOpen(server),
            child: Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: panelCardBorder),
              ),
              child: Row(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: Image.asset('assets/paper.png', width: 52, height: 52, fit: BoxFit.cover),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(server.name, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                        const SizedBox(height: 6),
                        Text(
                          'Paper ${server.paperVersion ?? 'n/d'}   ·   :${server.port}   ·   ${server.onlinePlayers}/${server.maxPlayers}',
                          style: const TextStyle(color: panelMuted),
                        ),
                      ],
                    ),
                  ),
                  Icon(Icons.circle, size: 10, color: online ? panelOnline : panelMuted),
                  const SizedBox(width: 8),
                  SizedBox(width: 110, child: Text(statusLabel(server.status), style: TextStyle(color: online ? panelOnline : panelMuted))),
                  if (running) ...[
                    FilledButton(
                      style: FilledButton.styleFrom(backgroundColor: const Color(0xFFC62828), foregroundColor: Colors.white),
                      onPressed: () => onStop(server),
                      child: const Text('Ferma'),
                    ),
                    const SizedBox(width: 8),
                    FilledButton(
                      style: FilledButton.styleFrom(backgroundColor: const Color(0xFFEF6C00), foregroundColor: Colors.white),
                      onPressed: onRestart == null ? null : () => onRestart!(server),
                      child: const Text('Riavvia'),
                    ),
                  ] else if (server.status != ServerStatus.stopping)
                    FilledButton(
                      style: FilledButton.styleFrom(backgroundColor: const Color(0xFF2E7D32), foregroundColor: Colors.white),
                      onPressed: () => onStart(server),
                      child: const Text('Avvia'),
                    ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
