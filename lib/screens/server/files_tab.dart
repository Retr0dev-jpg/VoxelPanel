// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/server.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/code_editor.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

/// Config file shortcuts, a file browser confined to the server folder and a text editor.
class FilesTab extends StatefulWidget {
  const FilesTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<FilesTab> createState() => _FilesTabState();
}

class _FilesTabState extends State<FilesTab> {
  var _browsing = false;
  var _directory = '';
  Future<List<String>>? _configs;
  Future<List<FileEntry>>? _entries;
  String? _openFile;
  HighlightingController? _editor;
  var _dirty = false;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void dispose() {
    _editor?.dispose();
    super.dispose();
  }

  void _reload() {
    setState(() {
      _configs = listConfigFiles(id: widget.serverId);
      _entries = listDirectory(id: widget.serverId, relative: _directory);
    });
  }

  void _navigate(String directory) {
    setState(() {
      _directory = directory;
      _entries = listDirectory(id: widget.serverId, relative: directory);
    });
  }

  Future<void> _open(String relative) async {
    if (_dirty && !await _confirmDiscard()) {
      return;
    }
    try {
      final content = await readTextFile(id: widget.serverId, relative: relative);
      if (!mounted) {
        return;
      }
      setState(() {
        _editor?.dispose();
        _editor = HighlightingController(text: content, language: languageOf(relative));
        _openFile = relative;
        _dirty = false;
      });
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    }
  }

  Future<bool> _confirmDiscard() {
    final l = context.l10n;
    return confirmAction(context, title: l.discardChangesTitle, message: l.discardChangesMessage, confirmLabel: l.discard, destructive: true);
  }

  Future<void> _closeEditor() async {
    if (_dirty && !await _confirmDiscard()) {
      return;
    }
    setState(() {
      _openFile = null;
      _dirty = false;
    });
  }

  Future<void> _saveFile() async {
    final file = _openFile;
    final editor = _editor;
    if (file == null || editor == null) {
      return;
    }
    final l = context.l10n;
    if (await runGuarded(context, () => writeTextFile(id: widget.serverId, relative: file, content: editor.text), success: widget.running ? l.fileSavedRestart : l.fileSaved)) {
      setState(() => _dirty = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    if (_openFile != null) {
      return _editorView(context);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Row(
            children: [
              SegmentedButton<bool>(
                segments: [
                  ButtonSegment(value: false, icon: const Icon(Icons.settings_suggest_outlined), label: Text(l.configFiles)),
                  ButtonSegment(value: true, icon: const Icon(Icons.folder_outlined), label: Text(l.fileBrowser)),
                ],
                selected: {_browsing},
                onSelectionChanged: (value) => setState(() => _browsing = value.first),
              ),
              const Spacer(),
              IconButton(tooltip: l.refresh, onPressed: _reload, icon: const Icon(Icons.refresh)),
            ],
          ),
        ),
        Expanded(child: _browsing ? _browser(context) : _configList(context)),
      ],
    );
  }

  Widget _configList(BuildContext context) {
    final l = context.l10n;
    return FutureBuilder<List<String>>(
      future: _configs,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return ErrorState(error: snapshot.error!, onRetry: _reload);
        }
        final files = snapshot.data;
        if (files == null) {
          return const Center(child: CircularProgressIndicator());
        }
        if (files.isEmpty) {
          return EmptyState(icon: Icons.description_outlined, message: l.noConfigFiles);
        }
        return ListView(
          padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
          children: [
            for (final file in files)
              ListTile(
                leading: const Icon(Icons.description_outlined),
                title: Text(file, style: const TextStyle(fontFamily: 'monospace')),
                trailing: const Icon(Icons.edit_outlined),
                onTap: () => _open(file),
              ),
          ],
        );
      },
    );
  }

  Widget _browser(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final parts = _directory.isEmpty ? <String>[] : _directory.split('/');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 28),
          child: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 4,
            runSpacing: 4,
            children: [
              TextButton.icon(onPressed: () => _navigate(''), icon: const Icon(Icons.home_outlined, size: 18), label: Text(l.serverFolderShort)),
              for (var index = 0; index < parts.length; index++) ...[
                Icon(Icons.chevron_right, size: 16, color: colors.muted),
                TextButton(onPressed: () => _navigate(parts.sublist(0, index + 1).join('/')), child: Text(parts[index])),
              ],
              const SizedBox(width: 12),
              OutlinedButton.icon(onPressed: _newFolder, icon: const Icon(Icons.create_new_folder_outlined, size: 18), label: Text(l.newFolder)),
              OutlinedButton.icon(onPressed: _importFiles, icon: const Icon(Icons.upload_file_outlined, size: 18), label: Text(l.importFiles)),
            ],
          ),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: FutureBuilder<List<FileEntry>>(
            future: _entries,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                return ErrorState(error: snapshot.error!, onRetry: () => _navigate(_directory));
              }
              final entries = snapshot.data;
              if (entries == null) {
                return const Center(child: CircularProgressIndicator());
              }
              if (entries.isEmpty) {
                return EmptyState(icon: Icons.folder_open, message: l.emptyFolder);
              }
              return ListView.builder(
                padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
                itemCount: entries.length,
                itemBuilder: (context, index) {
                  final entry = entries[index];
                  final modified = DateTime.fromMillisecondsSinceEpoch(entry.modifiedMs).toLocal().toString().substring(0, 16);
                  return ListTile(
                    dense: true,
                    leading: Icon(entry.isDir ? Icons.folder : (entry.editable ? Icons.description_outlined : Icons.insert_drive_file_outlined), color: entry.isDir ? colors.accent : null),
                    title: Text(entry.name),
                    subtitle: Text(entry.isDir ? modified : '${formatBytes(entry.sizeBytes)} · $modified'),
                    onTap: entry.isDir ? () => _navigate(entry.relative) : (entry.editable ? () => _open(entry.relative) : null),
                    trailing: PopupMenuButton<String>(
                      onSelected: (action) => _entryAction(action, entry),
                      itemBuilder: (context) => [
                        if (entry.editable) PopupMenuItem(value: 'edit', child: Text(l.edit)),
                        PopupMenuItem(value: 'rename', child: Text(l.rename)),
                        PopupMenuItem(value: 'export', child: Text(l.exportCopy)),
                        PopupMenuItem(value: 'delete', child: Text(l.delete, style: TextStyle(color: colors.danger))),
                      ],
                    ),
                  );
                },
              );
            },
          ),
        ),
      ],
    );
  }

  Future<String?> _askName(String title, String initial) async {
    final controller = TextEditingController(text: initial);
    final l = context.l10n;
    final value = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: TextField(controller: controller, autofocus: true, onSubmitted: (text) => Navigator.pop(context, text)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel)),
          FilledButton(onPressed: () => Navigator.pop(context, controller.text), child: Text(l.confirm)),
        ],
      ),
    );
    controller.dispose();
    final trimmed = value?.trim();
    return trimmed == null || trimmed.isEmpty || trimmed.contains('/') || trimmed.contains('\\') ? null : trimmed;
  }

  String _join(String name) => _directory.isEmpty ? name : '$_directory/$name';

  Future<void> _newFolder() async {
    final name = await _askName(context.l10n.newFolder, '');
    if (name != null && mounted && await runGuarded(context, () => createServerDirectory(id: widget.serverId, relative: _join(name)))) {
      _navigate(_directory);
    }
  }

  Future<void> _importFiles() async {
    final files = await FilePicker.pickFiles(dialogTitle: context.l10n.importFiles);
    final paths = files.map((file) => file.path).whereType<String>().toList();
    if (paths.isEmpty || !mounted) {
      return;
    }
    if (await runGuarded(context, () => importServerFiles(id: widget.serverId, relativeDir: _directory, sources: paths))) {
      _navigate(_directory);
    }
  }

  Future<void> _entryAction(String action, FileEntry entry) async {
    final l = context.l10n;
    switch (action) {
      case 'edit':
        await _open(entry.relative);
      case 'rename':
        final name = await _askName(l.rename, entry.name);
        if (name != null && mounted && await runGuarded(context, () => renameServerPath(id: widget.serverId, from: entry.relative, to: _join(name)))) {
          _navigate(_directory);
        }
      case 'export':
        final destination = await FilePicker.getDirectoryPath(dialogTitle: l.exportCopy);
        if (destination != null && mounted) {
          await runGuarded(context, () => exportServerPath(id: widget.serverId, relative: entry.relative, destinationDir: destination), success: l.exported);
        }
      case 'delete':
        final ok = await confirmAction(context, title: l.deletePathTitle(entry.name), message: l.deletePathMessage, destructive: true, confirmLabel: l.delete);
        if (ok && mounted && await runGuarded(context, () => deleteServerPath(id: widget.serverId, relative: entry.relative))) {
          _navigate(_directory);
        }
    }
  }

  Widget _editorView(BuildContext context) {
    final l = context.l10n;
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 12, 28, 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              IconButton(tooltip: l.back, onPressed: _closeEditor, icon: const Icon(Icons.arrow_back)),
              Expanded(child: Text(_openFile!, style: const TextStyle(fontFamily: 'monospace', fontWeight: FontWeight.w600), overflow: TextOverflow.ellipsis)),
              if (_dirty) Padding(padding: const EdgeInsets.only(right: 12), child: Text(l.unsaved, style: TextStyle(color: context.voxel.warning))),
              OutlinedButton(onPressed: () => _open(_openFile!), child: Text(l.reload)),
              const SizedBox(width: 8),
              FilledButton.icon(onPressed: _saveFile, icon: const Icon(Icons.save_outlined), label: Text(l.save)),
            ],
          ),
          const SizedBox(height: 12),
          Expanded(child: CodeEditor(controller: _editor!, onChanged: (_) => _dirty ? null : setState(() => _dirty = true))),
        ],
      ),
    );
  }
}
