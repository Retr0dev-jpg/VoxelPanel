// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

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
  var _busy = false;

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
    setState(() => _busy = true);
    await runGuarded(context, action, success: success);
    await _load();
    if (mounted) {
      setState(() => _busy = false);
    }
  }

  Future<String?> _ask(String title, {String initial = '', String? hint}) async {
    final controller = TextEditingController(text: initial);
    final l = context.l10n;
    final value = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: TextField(controller: controller, autofocus: true, decoration: InputDecoration(hintText: hint), onSubmitted: (text) => Navigator.pop(context, text)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel)),
          FilledButton(onPressed: () => Navigator.pop(context, controller.text), child: Text(l.confirm)),
        ],
      ),
    );
    controller.dispose();
    return value?.trim();
  }

  Future<void> _import({required bool zip}) async {
    final l = context.l10n;
    final source = zip
        ? (await FilePicker.pickFiles(dialogTitle: l.importWorld, type: FileType.custom, allowedExtensions: const ['zip'])).firstOrNull?.path
        : await FilePicker.getDirectoryPath(dialogTitle: l.importWorld);
    if (source == null || !mounted) {
      return;
    }
    final suggested = source.split(RegExp(r'[\\/]')).last.replaceAll('.zip', '');
    final name = await _ask(l.worldName, initial: suggested);
    if (name == null || name.isEmpty || !mounted) {
      return;
    }
    await _act(() => importWorld(id: widget.serverId, sourcePath: source, name: name), success: l.worldImported);
  }

  Future<void> _backup(String group) async {
    await _act(() async {
      await for (final event in backupWorld(id: widget.serverId, name: group)) {
        if (event.error != null) {
          throw Exception(event.error);
        }
        if (event.done && mounted) {
          showMessage(context, event.message);
        }
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final worlds = _worlds;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (worlds == null) {
      return const Center(child: CircularProgressIndicator());
    }
    final groups = <String, List<WorldInfo>>{};
    for (final world in worlds) {
      groups.putIfAbsent(world.group, () => []).add(world);
    }
    final locked = widget.running || _busy;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              FilledButton.tonalIcon(onPressed: locked ? null : () => _import(zip: true), icon: const Icon(Icons.folder_zip_outlined), label: Text(l.importWorldZip)),
              OutlinedButton.icon(onPressed: locked ? null : () => _import(zip: false), icon: const Icon(Icons.drive_folder_upload_outlined), label: Text(l.importWorldFolder)),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
            ],
          ),
        ),
        if (widget.running) Padding(padding: const EdgeInsets.symmetric(horizontal: 28), child: Text(l.stopToEditWorlds, style: TextStyle(color: context.voxel.warning))),
        if (_busy) const LinearProgressIndicator(),
        Expanded(
          child: groups.isEmpty
              ? EmptyState(icon: Icons.public_outlined, message: l.noWorlds)
              : ListView(
                  padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
                  children: [
                    for (final entry in groups.entries) ...[
                      _WorldGroupCard(
                        group: entry.key,
                        worlds: entry.value,
                        locked: locked,
                        onActivate: () => _act(() => setActiveWorld(id: widget.serverId, name: entry.key), success: l.restartToApply),
                        onOpen: (world) => runGuarded(context, () => openInExplorer(path: world.path)),
                        onBackup: () => _backup(entry.key),
                        onRename: () async {
                          final name = await _ask(l.renameWorld, initial: entry.key);
                          if (name != null && name.isNotEmpty && name != entry.key) {
                            await _act(() => renameWorld(id: widget.serverId, name: entry.key, newName: name));
                          }
                        },
                        onReset: () async {
                          final ok = await confirmAction(context, title: l.resetWorldTitle(entry.key), message: l.resetWorldMessage, destructive: true, confirmLabel: l.reset);
                          if (!ok || !mounted) {
                            return;
                          }
                          final seed = await _ask(l.newSeed, hint: l.seedHint);
                          if (seed == null) {
                            return;
                          }
                          await _act(() => resetWorld(id: widget.serverId, name: entry.key, seed: seed), success: l.worldReset);
                        },
                        onDelete: (world) async {
                          final ok = await confirmAction(context, title: l.deleteWorldTitle(world.name), message: l.deleteWorldMessage, destructive: true, confirmLabel: l.delete);
                          if (ok) {
                            await _act(() => deleteWorld(id: widget.serverId, name: world.name));
                          }
                        },
                      ),
                      const SizedBox(height: 12),
                    ],
                  ],
                ),
        ),
      ],
    );
  }
}

class _WorldGroupCard extends StatelessWidget {
  const _WorldGroupCard({
    required this.group,
    required this.worlds,
    required this.locked,
    required this.onActivate,
    required this.onOpen,
    required this.onBackup,
    required this.onRename,
    required this.onReset,
    required this.onDelete,
  });

  final String group;
  final List<WorldInfo> worlds;
  final bool locked;
  final VoidCallback onActivate;
  final ValueChanged<WorldInfo> onOpen;
  final VoidCallback onBackup;
  final VoidCallback onRename;
  final VoidCallback onReset;
  final ValueChanged<WorldInfo> onDelete;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final active = worlds.any((world) => world.active);
    final total = worlds.fold<int>(0, (sum, world) => sum + world.sizeBytes);
    return PanelCard(
      title: active ? l.worldActive(group) : group,
      subtitle: formatBytes(total),
      icon: Icons.public,
      trailing: Wrap(
        children: [
          IconButton(tooltip: l.setActiveWorld, onPressed: active ? null : onActivate, icon: Icon(Icons.check_circle_outline, color: active ? colors.online : null)),
          IconButton(tooltip: l.backupWorld, onPressed: onBackup, icon: const Icon(Icons.archive_outlined)),
          IconButton(tooltip: l.renameWorld, onPressed: locked ? null : onRename, icon: const Icon(Icons.edit_outlined)),
          IconButton(tooltip: l.reset, onPressed: locked ? null : onReset, icon: const Icon(Icons.restart_alt)),
        ],
      ),
      child: Column(
        children: [
          for (final world in worlds)
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              leading: Icon(switch (world.dimension) {
                WorldDimension.overworld => Icons.landscape_outlined,
                WorldDimension.nether => Icons.local_fire_department_outlined,
                WorldDimension.end => Icons.dark_mode_outlined,
              }),
              title: Text('${switch (world.dimension) {
                WorldDimension.overworld => l.dimensionOverworld,
                WorldDimension.nether => l.dimensionNether,
                WorldDimension.end => l.dimensionEnd,
              }} · ${world.name}'),
              subtitle: Text('${formatBytes(world.sizeBytes)} · ${DateTime.fromMillisecondsSinceEpoch(world.modifiedMs).toLocal().toString().substring(0, 16)}'),
              trailing: Wrap(
                children: [
                  IconButton(tooltip: l.openFolder, onPressed: () => onOpen(world), icon: const Icon(Icons.folder_open)),
                  IconButton(tooltip: l.delete, onPressed: locked ? null : () => onDelete(world), icon: const Icon(Icons.delete_outline)),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
