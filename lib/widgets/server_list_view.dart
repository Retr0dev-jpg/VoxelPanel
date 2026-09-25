// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';

class ServerListView extends StatelessWidget {
  const ServerListView({
    super.key,
    required this.servers,
    required this.onOpen,
    required this.onStart,
    required this.onStop,
    required this.onRestart,
    this.selection,
    this.onToggleSelected,
  });

  final List<ServerSummary> servers;
  final ValueChanged<ServerSummary> onOpen;
  final ValueChanged<ServerSummary> onStart;
  final ValueChanged<ServerSummary> onStop;
  final ValueChanged<ServerSummary> onRestart;

  /// When non-null the list is in multi-select mode.
  final Set<String>? selection;
  final ValueChanged<ServerSummary>? onToggleSelected;

  @override
  Widget build(BuildContext context) {
    return ListView.separated(
      padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
      itemCount: servers.length,
      separatorBuilder: (context, index) => const SizedBox(height: 12),
      itemBuilder: (context, index) {
        final server = servers[index];
        return ServerCard(
          server: server,
          selected: selection?.contains(server.id),
          onTap: selection == null ? () => onOpen(server) : () => onToggleSelected?.call(server),
          onStart: () => onStart(server),
          onStop: () => onStop(server),
          onRestart: () => onRestart(server),
        );
      },
    );
  }
}

class ServerCard extends ConsumerWidget {
  const ServerCard({
    super.key,
    required this.server,
    required this.onTap,
    required this.onStart,
    required this.onStop,
    required this.onRestart,
    this.selected,
  });

  final ServerSummary server;
  final bool? selected;
  final VoidCallback onTap;
  final VoidCallback onStart;
  final VoidCallback onStop;
  final VoidCallback onRestart;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.voxel;
    final l = context.l10n;
    final runtime = ref.watch(serverRuntimeProvider(server.id));
    final status = runtime?.status ?? ServerStatus.stopped;
    final players = runtime?.players.length ?? 0;
    final statusColor = switch (status) {
      ServerStatus.running => colors.online,
      ServerStatus.starting || ServerStatus.stopping => colors.warning,
      ServerStatus.stopped => runtime?.crashed == true ? colors.danger : colors.muted,
    };
    final statusText = runtime?.crashed == true && status == ServerStatus.stopped ? l.statusCrashed : statusLabel(context, status);
    return Material(
      color: colors.card,
      borderRadius: BorderRadius.circular(16),
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: selected == true ? colors.accent : colors.cardBorder, width: selected == true ? 2 : 1),
          ),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final narrow = constraints.maxWidth < 640;
              final info = Row(
                children: [
                  if (selected != null) Checkbox(value: selected, onChanged: (_) => onTap()),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: Image.asset('assets/paper.png', width: 52, height: 52, fit: BoxFit.cover),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(server.name, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700), overflow: TextOverflow.ellipsis),
                        const SizedBox(height: 6),
                        Text(
                          l.serverCardLine(server.paperVersion ?? l.notAvailable, server.port, players, server.maxPlayers),
                          style: TextStyle(color: colors.muted),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                ],
              );
              final state = Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.circle, size: 10, color: statusColor),
                  const SizedBox(width: 8),
                  Text(statusText, style: TextStyle(color: statusColor)),
                ],
              );
              final actions = selected != null ? const SizedBox.shrink() : _actions(context, status);
              if (narrow) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    info,
                    const SizedBox(height: 12),
                    Row(children: [state, const Spacer(), actions]),
                  ],
                );
              }
              return Row(
                children: [
                  Expanded(child: info),
                  SizedBox(width: 130, child: state),
                  actions,
                ],
              );
            },
          ),
        ),
      ),
    );
  }

  Widget _actions(BuildContext context, ServerStatus status) {
    final colors = context.voxel;
    final l = context.l10n;
    if (status == ServerStatus.stopping) {
      return const SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (status.isActive) {
      return Wrap(
        spacing: 8,
        children: [
          FilledButton(style: actionButtonStyle(colors.stop), onPressed: onStop, child: Text(l.actionStop)),
          FilledButton(style: actionButtonStyle(colors.restart), onPressed: onRestart, child: Text(l.actionRestart)),
        ],
      );
    }
    return FilledButton(style: actionButtonStyle(colors.start), onPressed: onStart, child: Text(l.actionStart));
  }
}
