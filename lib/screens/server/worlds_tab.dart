// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

class WorldsTab extends StatefulWidget {
  const WorldsTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<WorldsTab> createState() => _WorldsTabState();
}

class _WorldsTabState extends State<WorldsTab> {
  List<WorldInfo>? _worlds;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final worlds = await listWorlds(id: widget.serverId);
      if (mounted) {
        setState(() {
          _worlds = worlds;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _act(Future<void> Function() action, {String? success}) async {
    await runGuarded(context, action, success: success);
    await _load();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final worlds = _worlds;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (worlds == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (worlds.isEmpty) {
      return EmptyState(icon: Icons.public_outlined, message: l.noWorlds);
    }
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(20, 12, 20, 20),
      itemCount: worlds.length,
      itemBuilder: (context, index) {
        final world = worlds[index];
        return ListTile(
          leading: Icon(Icons.public, color: world.active ? colors.online : colors.muted),
          title: Text(world.active ? l.worldActive(world.name) : world.name),
          subtitle: Text('${formatBytes(world.sizeBytes)} · ${DateTime.fromMillisecondsSinceEpoch(world.modifiedMs).toLocal().toString().substring(0, 16)}'),
          trailing: Wrap(
            children: [
              IconButton(
                tooltip: l.openFolder,
                onPressed: () => runGuarded(context, () => openInExplorer(path: world.path)),
                icon: const Icon(Icons.folder_open),
              ),
              IconButton(
                tooltip: l.setActiveWorld,
                onPressed: world.active ? null : () => _act(() => setActiveWorld(id: widget.serverId, name: world.name), success: l.restartToApply),
                icon: const Icon(Icons.check),
              ),
              IconButton(
                tooltip: l.delete,
                onPressed: widget.running
                    ? null
                    : () async {
                        final ok = await confirmAction(context, title: l.deleteWorldTitle(world.name), message: l.deleteWorldMessage, destructive: true, confirmLabel: l.delete);
                        if (ok) {
                          await _act(() => deleteWorld(id: widget.serverId, name: world.name));
                        }
                      },
                icon: const Icon(Icons.delete_outline),
              ),
            ],
          ),
        );
      },
    );
  }
}
