// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

enum _PropertyKind { text, toggle, choice }

const _choices = {
  'difficulty': ['peaceful', 'easy', 'normal', 'hard'],
  'gamemode': ['survival', 'creative', 'adventure', 'spectator'],
};

class _PropertyField {
  _PropertyField(this.key, String value)
    : kind = _kindFor(key, value),
      controller = TextEditingController(text: value),
      enabled = value.toLowerCase() == 'true';

  final String key;
  final _PropertyKind kind;
  final TextEditingController controller;
  bool enabled;

  static _PropertyKind _kindFor(String key, String value) {
    if (_choices[key]?.contains(value) ?? false) {
      return _PropertyKind.choice;
    }
    if (value == 'true' || value == 'false') {
      return _PropertyKind.toggle;
    }
    return _PropertyKind.text;
  }

  String get value => kind == _PropertyKind.toggle ? (enabled ? 'true' : 'false') : controller.text;

  void dispose() => controller.dispose();
}

class PropertiesTab extends StatefulWidget {
  const PropertiesTab({super.key, required this.serverId});

  final String serverId;

  @override
  State<PropertiesTab> createState() => _PropertiesTabState();
}

class _PropertiesTabState extends State<PropertiesTab> {
  final _fields = <_PropertyField>[];
  Object? _error;
  var _ready = false;
  var _filter = '';

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
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final entries = await listProperties(id: widget.serverId);
      if (!mounted) {
        return;
      }
      setState(() {
        for (final field in _fields) {
          field.dispose();
        }
        _fields
          ..clear()
          ..addAll(entries.map((entry) => _PropertyField(entry.key, entry.value)));
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

  String _label(BuildContext context, String key) {
    final l = context.l10n;
    return switch (key) {
      'motd' => l.propMotd,
      'server-port' => l.propPort,
      'max-players' => l.propMaxPlayers,
      'difficulty' => l.propDifficulty,
      'gamemode' => l.propGamemode,
      'view-distance' => l.propViewDistance,
      'simulation-distance' => l.propSimulationDistance,
      'spawn-protection' => l.propSpawnProtection,
      'level-name' => l.propLevelName,
      'level-seed' => l.propLevelSeed,
      'online-mode' => l.propOnlineMode,
      'white-list' => l.propWhitelist,
      'pvp' => l.propPvp,
      _ => key,
    };
  }

  String _choiceLabel(BuildContext context, String value) {
    final l = context.l10n;
    return switch (value) {
      'peaceful' => l.difficultyPeaceful,
      'easy' => l.difficultyEasy,
      'normal' => l.difficultyNormal,
      'hard' => l.difficultyHard,
      'survival' => l.gamemodeSurvival,
      'creative' => l.gamemodeCreative,
      'adventure' => l.gamemodeAdventure,
      'spectator' => l.gamemodeSpectator,
      _ => value,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (_fields.isEmpty) {
      return EmptyState(icon: Icons.tune, message: l.propertiesMissing);
    }
    final visible = _fields.where((field) {
      final query = _filter.toLowerCase();
      return query.isEmpty || field.key.contains(query) || _label(context, field.key).toLowerCase().contains(query);
    }).toList();
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.propertiesSearch, isDense: true),
                  onChanged: (value) => setState(() => _filter = value.trim()),
                ),
              ),
              const SizedBox(width: 12),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
              const SizedBox(width: 4),
              FilledButton.icon(onPressed: _save, icon: const Icon(Icons.save_outlined), label: Text(l.save)),
            ],
          ),
        ),
        Expanded(
          child: ListView.separated(
            padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
            itemCount: visible.length,
            separatorBuilder: (context, index) => const SizedBox(height: 10),
            itemBuilder: (context, index) => _editor(context, visible[index]),
          ),
        ),
      ],
    );
  }

  Widget _editor(BuildContext context, _PropertyField field) {
    final label = _label(context, field.key);
    final helper = label == field.key ? null : field.key;
    switch (field.kind) {
      case _PropertyKind.toggle:
        return SwitchListTile(
          value: field.enabled,
          tileColor: context.voxel.field,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          title: Text(label),
          subtitle: helper == null ? null : Text(helper),
          onChanged: (value) => setState(() => field.enabled = value),
        );
      case _PropertyKind.choice:
        return DropdownButtonFormField<String>(
          initialValue: field.controller.text,
          decoration: InputDecoration(labelText: label, helperText: helper),
          items: [
            for (final option in _choices[field.key]!) DropdownMenuItem(value: option, child: Text(_choiceLabel(context, option))),
          ],
          onChanged: (value) {
            if (value != null) {
              field.controller.text = value;
            }
          },
        );
      case _PropertyKind.text:
        return TextField(controller: field.controller, decoration: InputDecoration(labelText: label, helperText: helper));
    }
  }

  Future<void> _save() async {
    await runGuarded(
      context,
      () => saveProperties(
        id: widget.serverId,
        entries: [for (final field in _fields) PropertyEntry(key: field.key, value: field.value)],
      ),
      success: context.l10n.propertiesSaved,
    );
  }
}
