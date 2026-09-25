// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:math' as math;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/screens/content/content_browser.dart';
import 'package:voxel_panel/screens/create/create_validation.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/content.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/rust/api/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

/// Creates a server from a `.mrpack`, a CurseForge zip or a modpack found in a catalogue.
class ModpackForm extends StatefulWidget {
  const ModpackForm({super.key, required this.onCreated, required this.onBusyChanged});

  final ValueChanged<String> onCreated;
  final ValueChanged<bool> onBusyChanged;

  @override
  State<ModpackForm> createState() => _ModpackFormState();
}

class _ModpackFormState extends State<ModpackForm> {
  late final List<ContentSourceKind> _sources = modpackSources();
  ContentSourceKind? _source;
  var _file = '';
  final _query = TextEditingController();
  final _name = TextEditingController();
  List<ContentProject> _results = [];
  ContentProject? _project;
  Future<List<ContentVersion>>? _versions;
  var _version = '';
  late final int _systemMb = math.max(systemMemoryMb().toInt(), 1024);
  late var _ramMaxMb = math.min(6144, _systemMb);
  var _eula = false;
  var _busy = false;
  var _searching = false;
  final _log = <String>[];

  @override
  void dispose() {
    _query.dispose();
    _name.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    final source = _source;
    if (source == null) {
      return;
    }
    setState(() => _searching = true);
    try {
      final page = await searchModpacks(source: source, query: _query.text, page: 0);
      if (mounted) {
        setState(() => _results = page.projects);
      }
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    } finally {
      if (mounted) {
        setState(() => _searching = false);
      }
    }
  }

  void _select(ContentProject project) {
    setState(() {
      _project = project;
      _version = '';
      _versions = modpackVersions(source: project.source, projectId: project.id)..ignore();
      if (_name.text.isEmpty) {
        _name.text = project.title;
      }
    });
  }

  Future<void> _create() async {
    final l = context.l10n;
    if (_source == null && _file.isEmpty) {
      showMessage(context, l.modpackChooseSource);
      return;
    }
    if (_source != null && _project == null) {
      showMessage(context, l.modpackChooseProject);
      return;
    }
    if (!_eula) {
      showMessage(context, l.issueEula);
      return;
    }
    setState(() {
      _busy = true;
      _log.clear();
    });
    widget.onBusyChanged(true);
    String? created;
    try {
      final stream = createFromModpack(
        request: ModpackRequest(
          name: _name.text.trim(),
          root: '',
          filePath: _source == null ? _file : '',
          source: _source,
          projectId: _project?.id ?? '',
          versionId: _version,
          javaHome: '',
          ramMin: memoryValue(math.min(2048, _ramMaxMb)),
          ramMax: memoryValue(_ramMaxMb),
          jvmFlags: jvmPresets().where((preset) => preset.preset == JvmPreset.aikar).firstOrNull?.flags ?? const [],
          acceptEula: true,
        ),
      );
      await for (final event in stream) {
        if (!mounted) {
          return;
        }
        setState(() {
          if (_log.isNotEmpty && event.fraction != null && !event.done && _log.last.startsWith('${event.stage}: ')) {
            _log[_log.length - 1] = '${event.stage}: ${event.message}';
          } else {
            _log.add('${event.stage}: ${event.message}');
          }
        });
        if (event.error != null) {
          throw Exception(event.error);
        }
        created = event.serverId ?? created;
      }
    } catch (error) {
      if (mounted) {
        setState(() => _log.add(describeError(context, error)));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        widget.onBusyChanged(false);
      }
    }
    if (created != null && mounted) {
      widget.onCreated(created);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return PanelCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.modpackSource, style: const TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: [
              ChoiceChip(label: Text(l.modpackFile), selected: _source == null, onSelected: _busy ? null : (_) => setState(() => _source = null)),
              for (final source in _sources)
                ChoiceChip(
                  label: Text(sourceLabel(source)),
                  selected: _source == source,
                  onSelected: _busy
                      ? null
                      : (_) {
                          setState(() {
                            _source = source;
                            _project = null;
                            _results = [];
                          });
                          _search();
                        },
                ),
            ],
          ),
          if (!_sources.contains(ContentSourceKind.curseForge)) Padding(padding: const EdgeInsets.only(top: 6), child: Text(l.curseforgeKeyHint, style: TextStyle(color: colors.muted, fontSize: 12))),
          const SizedBox(height: 12),
          if (_source == null)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.inventory_2_outlined),
              title: Text(_file.isEmpty ? l.modpackNoFile : _file),
              subtitle: Text(l.modpackFileHint),
              trailing: FilledButton.tonal(
                onPressed: _busy
                    ? null
                    : () async {
                        final files = await FilePicker.pickFiles(dialogTitle: l.modpackFile, type: FileType.custom, allowedExtensions: const ['mrpack', 'zip']);
                        final path = files.firstOrNull?.path;
                        if (path != null) {
                          setState(() => _file = path);
                        }
                      },
                child: Text(l.browse),
              ),
            )
          else ...[
            TextField(
              controller: _query,
              enabled: !_busy,
              decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.searchModpacks, suffixIcon: IconButton(onPressed: _search, icon: const Icon(Icons.arrow_forward))),
              onSubmitted: (_) => _search(),
            ),
            if (_searching) const LinearProgressIndicator(),
            SizedBox(
              height: 220,
              child: ListView(
                children: [
                  for (final project in _results)
                    ListTile(
                      selected: _project?.id == project.id,
                      leading: ProjectIcon(project.iconUrl, size: 36),
                      title: Text(project.title),
                      subtitle: Text(project.description, maxLines: 1, overflow: TextOverflow.ellipsis),
                      trailing: Text('↓ ${compactCount(project.downloads)}'),
                      onTap: _busy ? null : () => _select(project),
                    ),
                ],
              ),
            ),
            if (_versions != null)
              FutureBuilder<List<ContentVersion>>(
                future: _versions,
                builder: (context, snapshot) {
                  final versions = snapshot.data ?? const <ContentVersion>[];
                  return DropdownButtonFormField<String>(
                    key: ValueKey('pack-${_project?.id}-${versions.length}'),
                    initialValue: _version,
                    isExpanded: true,
                    decoration: InputDecoration(labelText: l.stepVersion, helperText: snapshot.connectionState == ConnectionState.waiting ? l.loading : null),
                    items: [
                      DropdownMenuItem(value: '', child: Text(l.latestVersionShort)),
                      for (final version in versions.take(40)) DropdownMenuItem(value: version.id, child: Text('${version.name} · ${version.gameVersions.take(3).join(', ')}', overflow: TextOverflow.ellipsis)),
                    ],
                    onChanged: _busy ? null : (value) => setState(() => _version = value ?? ''),
                  );
                },
              ),
          ],
          const SizedBox(height: 12),
          TextField(controller: _name, enabled: !_busy, decoration: InputDecoration(labelText: l.serverName, hintText: l.modpackNameHint)),
          const SizedBox(height: 12),
          Text('${l.ramMax}: ${memoryValue(_ramMaxMb)}'),
          Slider(
            value: _ramMaxMb.toDouble().clamp(1024, _systemMb.toDouble()),
            min: 1024,
            max: math.max(_systemMb.toDouble(), 1025),
            divisions: math.max(1, (_systemMb - 1024) ~/ 512),
            label: memoryValue(_ramMaxMb),
            onChanged: _busy ? null : (value) => setState(() => _ramMaxMb = ((value / 512).round() * 512).clamp(1024, _systemMb)),
          ),
          Text(l.modpackRamHint, style: TextStyle(color: colors.muted, fontSize: 12)),
          CheckboxListTile(contentPadding: EdgeInsets.zero, value: _eula, onChanged: _busy ? null : (value) => setState(() => _eula = value ?? false), title: Text(l.eulaCheckbox)),
          FilledButton.icon(
            onPressed: _busy ? null : _create,
            icon: _busy ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.rocket_launch_outlined),
            label: Text(_busy ? l.installing : l.createFromModpack),
          ),
          if (_log.isNotEmpty) ...[
            const SizedBox(height: 16),
            Container(
              constraints: const BoxConstraints(maxHeight: 260),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12)),
              child: SingleChildScrollView(
                reverse: true,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [for (final line in _log) Text(line, style: const TextStyle(fontFamily: 'monospace', fontSize: 12, color: Color(0xFFDDDDDD)))],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
