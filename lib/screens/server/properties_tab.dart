// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/server.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/code_editor.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

String propertyGroupLabel(AppLocalizations l, PropertyGroup? group) => switch (group) {
  PropertyGroup.general => l.groupGeneral,
  PropertyGroup.gameplay => l.groupGameplay,
  PropertyGroup.world => l.groupWorld,
  PropertyGroup.network => l.groupNetwork,
  PropertyGroup.performance => l.groupPerformance,
  PropertyGroup.administration => l.groupAdministration,
  PropertyGroup.queryRcon => l.groupQueryRcon,
  PropertyGroup.resourcePack => l.groupResourcePack,
  null => l.groupOther,
};

class _Field {
  _Field(this.key, String value, this.schema)
    : original = value,
      controller = TextEditingController(text: value),
      enabled = value == 'true';

  final String key;
  final String original;
  final PropertySchema? schema;
  final TextEditingController controller;
  bool enabled;

  bool get isBool => schema?.kind == PropertyKind.boolean || (schema == null && (original == 'true' || original == 'false'));
  String get value => isBool ? (enabled ? 'true' : 'false') : controller.text;
  bool get changed => value != original;

  void dispose() => controller.dispose();
}

class PropertiesTab extends StatefulWidget {
  const PropertiesTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<PropertiesTab> createState() => _PropertiesTabState();
}

class _PropertiesTabState extends State<PropertiesTab> {
  final _fields = <_Field>[];
  late final Map<String, PropertySchema> _schema = {for (final entry in propertiesSchema()) entry.key: entry};
  final _raw = HighlightingController(language: CodeLanguage.properties);
  Object? _error;
  var _ready = false;
  var _rawMode = false;
  var _filter = '';
  var _savedWhileRunning = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final field in _fields) {
      field.dispose();
    }
    _raw.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final entries = await listProperties(id: widget.serverId);
      final raw = await readTextFile(id: widget.serverId, relative: 'server.properties');
      if (!mounted) {
        return;
      }
      setState(() {
        for (final field in _fields) {
          field.dispose();
        }
        _fields
          ..clear()
          ..addAll(entries.map((entry) => _Field(entry.key, entry.value, _schema[entry.key])));
        _raw.text = raw;
        _error = null;
        _ready = true;
      });
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error;
          _ready = true;
        });
      }
    }
  }

  Future<void> _save() async {
    final l = context.l10n;
    final ok = await runGuarded(context, () async {
      if (_rawMode) {
        await writeTextFile(id: widget.serverId, relative: 'server.properties', content: _raw.text);
      } else {
        await saveProperties(id: widget.serverId, entries: [for (final field in _fields) PropertyEntry(key: field.key, value: field.value)]);
      }
    }, success: widget.running ? l.propertiesSavedRestart : l.propertiesSavedOk);
    if (ok) {
      setState(() => _savedWhileRunning = widget.running);
      await _load();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Row(
            children: [
              if (!_rawMode)
                Expanded(
                  child: TextField(
                    decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.propertiesSearch, isDense: true),
                    onChanged: (value) => setState(() => _filter = value.trim().toLowerCase()),
                  ),
                )
              else
                const Spacer(),
              const SizedBox(width: 12),
              SegmentedButton<bool>(
                segments: [
                  ButtonSegment(value: false, icon: const Icon(Icons.view_list), label: Text(l.propertiesForm)),
                  ButtonSegment(value: true, icon: const Icon(Icons.code), label: Text(l.propertiesRaw)),
                ],
                selected: {_rawMode},
                onSelectionChanged: (value) async {
                  await _load();
                  setState(() => _rawMode = value.first);
                },
              ),
              const SizedBox(width: 8),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
              FilledButton.icon(onPressed: _save, icon: const Icon(Icons.save_outlined), label: Text(l.save)),
            ],
          ),
        ),
        if (widget.running && _savedWhileRunning)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28),
            child: Text(l.restartRequired, style: TextStyle(color: colors.warning)),
          ),
        Expanded(child: _rawMode ? Padding(padding: const EdgeInsets.fromLTRB(28, 8, 28, 28), child: CodeEditor(controller: _raw)) : _form(context)),
      ],
    );
  }

  Widget _form(BuildContext context) {
    final l = context.l10n;
    if (_fields.isEmpty) {
      return EmptyState(icon: Icons.tune, message: l.propertiesMissing);
    }
    final english = Localizations.localeOf(context).languageCode == 'en';
    final visible = _fields.where((field) {
      final description = english ? field.schema?.descriptionEn : field.schema?.descriptionIt;
      return _filter.isEmpty || field.key.contains(_filter) || (description ?? '').toLowerCase().contains(_filter);
    });
    final groups = <PropertyGroup?, List<_Field>>{};
    for (final field in visible) {
      groups.putIfAbsent(field.schema?.group, () => []).add(field);
    }
    final ordered = [...PropertyGroup.values.where(groups.containsKey), if (groups.containsKey(null)) null];
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
      children: [
        for (final group in ordered) ...[
          PanelCard(
            title: propertyGroupLabel(l, group),
            child: Column(
              children: [for (final field in groups[group]!) _editor(context, field, english)],
            ),
          ),
          const SizedBox(height: 12),
        ],
      ],
    );
  }

  Widget _editor(BuildContext context, _Field field, bool english) {
    final colors = context.voxel;
    final schema = field.schema;
    final description = schema == null ? null : (english ? schema.descriptionEn : schema.descriptionIt);
    final label = Row(
      children: [
        Flexible(child: Text(field.key, style: const TextStyle(fontFamily: 'monospace', fontWeight: FontWeight.w600))),
        if (field.changed) ...[const SizedBox(width: 6), Icon(Icons.circle, size: 8, color: colors.warning)],
      ],
    );
    final subtitle = description == null ? null : Text(description, style: TextStyle(color: colors.muted, fontSize: 12));
    if (field.isBool) {
      return SwitchListTile(
        contentPadding: EdgeInsets.zero,
        title: label,
        subtitle: subtitle,
        value: field.enabled,
        onChanged: (value) => setState(() => field.enabled = value),
      );
    }
    Widget input;
    if (schema?.kind == PropertyKind.choice && schema!.options.contains(field.controller.text)) {
      input = DropdownButtonFormField<String>(
        initialValue: field.controller.text,
        isDense: true,
        isExpanded: true,
        items: [for (final option in schema.options) DropdownMenuItem(value: option, child: Text(option.replaceAll(r'\:', ':')))],
        onChanged: (value) => setState(() => field.controller.text = value ?? field.controller.text),
      );
    } else {
      final numeric = schema?.kind == PropertyKind.integer;
      input = TextField(
        controller: field.controller,
        obscureText: schema?.kind == PropertyKind.secret,
        keyboardType: numeric ? TextInputType.number : null,
        inputFormatters: numeric ? [FilteringTextInputFormatter.allow(RegExp(r'^-?\d*'))] : null,
        onChanged: (_) => setState(() {}),
        decoration: InputDecoration(
          isDense: true,
          hintText: schema?.defaultValue,
          helperText: numeric && schema?.min != null ? '${schema!.min} – ${schema.max}' : null,
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final text = Column(crossAxisAlignment: CrossAxisAlignment.start, children: [label, ?subtitle]);
          if (constraints.maxWidth < 620) {
            return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [text, const SizedBox(height: 6), input]);
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(child: text),
              const SizedBox(width: 16),
              SizedBox(width: 300, child: input),
            ],
          );
        },
      ),
    );
  }
}
