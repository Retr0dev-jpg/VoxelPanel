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

/// Plugins or mods of a server, depending on [kind].
class AddonsTab extends StatefulWidget {
  const AddonsTab({super.key, required this.serverId, required this.running, required this.kind});

  final String serverId;
  final bool running;
  final AddonKind kind;

  @override
  State<AddonsTab> createState() => _AddonsTabState();
}

class _AddonsTabState extends State<AddonsTab> {
  List<AddonInfo> _plugins = [];
  Object? _error;
  var _busy = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final plugins = await listAddons(id: widget.serverId, kind: widget.kind);
      if (mounted) {
        setState(() {
          _plugins = plugins;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _act(Future<void> Function() action) async {
    setState(() => _busy = true);
    await runGuarded(context, action);
    await _load();
    if (mounted) {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final locked = widget.running || _busy;
    final isMod = widget.kind == AddonKind.mod;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              FilledButton.icon(
                onPressed: locked ? null : _pickJar,
                icon: const Icon(Icons.upload_file),
                label: Text(isMod ? l.installModJar : l.installJar),
              ),
              FilledButton.tonalIcon(
                onPressed: locked ? null : () => _search(context),
                icon: const Icon(Icons.travel_explore),
                label: Text(l.searchModrinth),
              ),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
            ],
          ),
        ),
        if (widget.running)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Text(isMod ? l.stopToEditMods : l.stopToEditPlugins, style: TextStyle(color: context.voxel.warning)),
          ),
        if (_busy) const LinearProgressIndicator(),
        Expanded(child: _body(context, locked)),
      ],
    );
  }

  Widget _body(BuildContext context, bool locked) {
    final l = context.l10n;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (_plugins.isEmpty) {
      return EmptyState(icon: Icons.extension_outlined, message: widget.kind == AddonKind.mod ? l.noMods : l.noPlugins);
    }
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      itemCount: _plugins.length,
      itemBuilder: (context, index) {
        final plugin = _plugins[index];
        return ListTile(
          leading: Icon(Icons.extension, color: plugin.enabled ? context.voxel.accent : context.voxel.muted),
          title: Text(plugin.fileName),
          subtitle: Text(formatBytes(plugin.sizeBytes)),
          trailing: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Switch(
                value: plugin.enabled,
                onChanged: locked ? null : (value) => _act(() => setAddonEnabled(id: widget.serverId, kind: widget.kind, fileName: plugin.fileName, enabled: value)),
              ),
              IconButton(
                tooltip: l.delete,
                onPressed: locked
                    ? null
                    : () async {
                        final ok = await confirmAction(context, title: l.deletePluginTitle, message: plugin.fileName, destructive: true, confirmLabel: l.delete);
                        if (ok) {
                          await _act(() => deleteAddon(id: widget.serverId, kind: widget.kind, fileName: plugin.fileName));
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

  Future<void> _pickJar() async {
    final files = await FilePicker.pickFiles(dialogTitle: context.l10n.pluginJarTitle, type: FileType.custom, allowedExtensions: const ['jar']);
    final path = files.isEmpty ? null : files.first.path;
    if (path != null) {
      await _act(() => installAddonFile(id: widget.serverId, kind: widget.kind, sourcePath: path));
    }
  }

  Future<void> _search(BuildContext context) async {
    final projectId = await showDialog<String>(context: context, builder: (context) => _ModrinthDialog(serverId: widget.serverId, kind: widget.kind));
    if (projectId == null) {
      return;
    }
    await _act(() async {
      await for (final event in installModrinthProject(id: widget.serverId, kind: widget.kind, projectId: projectId)) {
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
    });
  }
}

class _ModrinthDialog extends StatefulWidget {
  const _ModrinthDialog({required this.serverId, required this.kind});

  final String serverId;
  final AddonKind kind;

  @override
  State<_ModrinthDialog> createState() => _ModrinthDialogState();
}

class _ModrinthDialogState extends State<_ModrinthDialog> {
  final _query = TextEditingController();
  List<ModrinthProject> _hits = [];
  var _loading = false;
  Object? _error;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  Future<void> _run(String query) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final hits = await searchModrinth(id: widget.serverId, kind: widget.kind, query: query);
      if (mounted) {
        setState(() => _hits = hits);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return AlertDialog(
      title: const Text('Modrinth'),
      content: SizedBox(
        width: 520,
        height: 380,
        child: Column(
          children: [
            TextField(
              controller: _query,
              autofocus: true,
              decoration: InputDecoration(labelText: widget.kind == AddonKind.mod ? l.searchMods : l.searchPlugins, suffixIcon: IconButton(onPressed: () => _run(_query.text), icon: const Icon(Icons.search))),
              onSubmitted: _run,
            ),
            const SizedBox(height: 8),
            if (_loading) const LinearProgressIndicator(),
            Expanded(
              child: _error != null
                  ? ErrorState(error: _error!)
                  : ListView(
                      children: [
                        for (final hit in _hits)
                          ListTile(
                            title: Text(hit.title),
                            subtitle: Text(hit.description, maxLines: 2, overflow: TextOverflow.ellipsis),
                            trailing: Text(l.downloadsCount(hit.downloads)),
                            onTap: () => Navigator.pop(context, hit.projectId),
                          ),
                      ],
                    ),
            ),
          ],
        ),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: Text(l.close))],
    );
  }
}
