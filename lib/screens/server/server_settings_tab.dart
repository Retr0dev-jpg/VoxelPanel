// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:io';
import 'dart:math' as math;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:voxel_panel/screens/create/create_validation.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/server.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/provider_icon.dart';

class ServerSettingsTab extends StatefulWidget {
  const ServerSettingsTab({super.key, required this.details, required this.running, required this.onChanged, required this.onChangeVersion});

  final ServerDetails details;
  final bool running;
  final VoidCallback onChanged;
  final VoidCallback onChangeVersion;

  @override
  State<ServerSettingsTab> createState() => _ServerSettingsTabState();
}

class _ServerSettingsTabState extends State<ServerSettingsTab> {
  ServerConfig? _config;
  Object? _error;
  var _saving = false;
  final _name = TextEditingController();
  final _flags = TextEditingController();
  final _stopCommand = TextEditingController();
  final _stopTimeout = TextEditingController();
  var _javaHome = '';
  var _ramMinMb = 1024;
  var _ramMaxMb = 2048;
  var _autostart = false;
  var _autoRestart = false;
  var _manageRcon = true;
  String _icon = '';
  var _iconVersion = 0;
  late final int _systemMb = math.max(systemMemoryMb().toInt(), 1024);
  late final Future<List<JavaRuntimeInfo>> _runtimes = listRuntimes();

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final controller in [_name, _flags, _stopCommand, _stopTimeout]) {
      controller.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final config = await getServerConfig(id: widget.details.id);
      final icon = await serverIconPath(id: widget.details.id);
      if (!mounted) {
        return;
      }
      setState(() {
        _config = config;
        _error = null;
        _name.text = config.name;
        _flags.text = config.jvmFlags.join('\n');
        _stopCommand.text = config.stopCommand;
        _stopTimeout.text = config.stopTimeoutSecs == 0 ? '' : '${config.stopTimeoutSecs}';
        _javaHome = config.javaHome;
        _ramMaxMb = (parseMemoryMb(config.ramMax) ?? 2048).clamp(512, math.max(_systemMb, 512));
        _ramMinMb = (parseMemoryMb(config.ramMin) ?? 1024).clamp(256, _ramMaxMb);
        _autostart = config.autostart;
        _autoRestart = config.autoRestart;
        _manageRcon = config.manageRcon;
        _icon = icon;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    final ok = await runGuarded(
      context,
      () => saveServerConfig(
        id: widget.details.id,
        config: ServerConfig(
          name: _name.text.trim(),
          javaHome: _javaHome,
          ramMin: memoryValue(_ramMinMb),
          ramMax: memoryValue(_ramMaxMb),
          jvmFlags: _flags.text.split(RegExp(r'\s+')).where((flag) => flag.isNotEmpty).toList(),
          stopCommand: _stopCommand.text.trim(),
          stopTimeoutSecs: int.tryParse(_stopTimeout.text) ?? 0,
          autostart: _autostart,
          autoRestart: _autoRestart,
          manageRcon: _manageRcon,
        ),
      ),
      success: context.l10n.serverSettingsSaved,
    );
    if (!mounted) {
      return;
    }
    setState(() => _saving = false);
    if (ok) {
      widget.onChanged();
      await _load();
    }
  }

  Future<void> _pickIcon() async {
    final files = await FilePicker.pickFiles(dialogTitle: context.l10n.serverIcon, type: FileType.image);
    final path = files.isEmpty ? null : files.first.path;
    if (path == null || !mounted) {
      return;
    }
    if (await runGuarded(context, () => setServerIcon(id: widget.details.id, sourcePath: path))) {
      imageCache.clear();
      setState(() => _iconVersion++);
      await _load();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (_config == null) {
      return const Center(child: CircularProgressIndicator());
    }
    final details = widget.details;
    final locked = widget.running;
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 12, 28, 28),
      children: [
        if (locked)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Text(l.stopToEditRuntime, style: TextStyle(color: colors.warning)),
          ),
        PanelCard(
          title: l.serverIdentity,
          icon: Icons.badge_outlined,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Column(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: _icon.isEmpty
                        ? ProviderIcon(details.provider, size: 64)
                        : Image.file(File(_icon), key: ValueKey('$_icon$_iconVersion'), width: 64, height: 64, filterQuality: FilterQuality.none),
                  ),
                  const SizedBox(height: 8),
                  TextButton(onPressed: _pickIcon, child: Text(l.changeIcon)),
                  if (_icon.isNotEmpty)
                    TextButton(
                      onPressed: () async {
                        if (await runGuarded(context, () => removeServerIcon(id: details.id))) {
                          await _load();
                        }
                      },
                      child: Text(l.removeIcon),
                    ),
                ],
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    TextField(controller: _name, decoration: InputDecoration(labelText: l.serverName)),
                    const SizedBox(height: 12),
                    Text(l.serverIconHint, style: TextStyle(color: colors.muted, fontSize: 12)),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        Expanded(child: Text('${providerName(details.provider)} ${details.mcVersion ?? ''}${details.build == null ? '' : ' · #${details.build}'}')),
                        OutlinedButton.icon(
                          onPressed: locked || details.provider == ProviderKind.custom ? null : widget.onChangeVersion,
                          icon: const Icon(Icons.system_update_alt),
                          label: Text(l.changeVersion),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        PanelCard(
          title: l.stepRuntime,
          icon: Icons.memory,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              FutureBuilder<List<JavaRuntimeInfo>>(
                future: _runtimes,
                builder: (context, snapshot) {
                  final runtimes = snapshot.data ?? const <JavaRuntimeInfo>[];
                  final options = {
                    for (final runtime in runtimes) runtime.path: runtime.major == 0 ? runtime.name : 'Java ${runtime.major} · ${runtime.name}',
                    if (_javaHome.isNotEmpty && !runtimes.any((runtime) => runtime.path == _javaHome)) _javaHome: _javaHome,
                  };
                  return DropdownButtonFormField<String>(
                    key: ValueKey('java-${options.length}'),
                    initialValue: options.containsKey(_javaHome) ? _javaHome : null,
                    isExpanded: true,
                    decoration: InputDecoration(labelText: l.javaRuntime, helperText: details.javaMajor == null ? null : l.requiresJava(details.javaMajor!)),
                    items: [for (final entry in options.entries) DropdownMenuItem(value: entry.key, child: Text(entry.value, overflow: TextOverflow.ellipsis))],
                    onChanged: locked ? null : (value) => setState(() => _javaHome = value ?? _javaHome),
                  );
                },
              ),
              const SizedBox(height: 16),
              Text('${l.ramMax}: ${memoryValue(_ramMaxMb)}'),
              Slider(
                value: _ramMaxMb.toDouble().clamp(512, _systemMb.toDouble()),
                min: 512,
                max: _systemMb.toDouble(),
                divisions: math.max(1, (_systemMb - 512) ~/ 256),
                label: memoryValue(_ramMaxMb),
                onChanged: locked
                    ? null
                    : (value) => setState(() {
                        _ramMaxMb = ((value / 256).round() * 256).clamp(512, _systemMb);
                        _ramMinMb = math.min(_ramMinMb, _ramMaxMb);
                      }),
              ),
              Text('${l.ramMin}: ${memoryValue(_ramMinMb)}'),
              Slider(
                value: _ramMinMb.toDouble().clamp(256, _ramMaxMb.toDouble()),
                min: 256,
                max: math.max(_ramMaxMb.toDouble(), 257),
                divisions: math.max(1, (_ramMaxMb - 256) ~/ 256),
                label: memoryValue(_ramMinMb),
                onChanged: locked ? null : (value) => setState(() => _ramMinMb = ((value / 256).round() * 256).clamp(256, _ramMaxMb)),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                children: [
                  for (final preset in jvmPresets())
                    ActionChip(
                      label: Text(switch (preset.preset) {
                        JvmPreset.aikar => l.jvmPresetAikar,
                        JvmPreset.g1 => l.jvmPresetG1,
                        JvmPreset.zgc => l.jvmPresetZgc,
                        JvmPreset.none => l.jvmPresetNone,
                      }),
                      onPressed: locked ? null : () => setState(() => _flags.text = preset.flags.join('\n')),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              TextField(
                controller: _flags,
                enabled: !locked,
                minLines: 3,
                maxLines: 8,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                decoration: InputDecoration(labelText: l.jvmFlags, helperText: l.jvmFlagsHint),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        PanelCard(
          title: l.serverBehaviour,
          icon: Icons.power_settings_new,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Expanded(child: TextField(controller: _stopCommand, decoration: InputDecoration(labelText: l.stopCommand, hintText: providerInfo(kind: details.provider).isProxy ? 'end' : 'stop'))),
                  const SizedBox(width: 12),
                  SizedBox(
                    width: 180,
                    child: TextField(
                      controller: _stopTimeout,
                      inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                      decoration: InputDecoration(labelText: l.settingStopTimeout, hintText: l.useLauncherDefault, suffixText: 's'),
                    ),
                  ),
                ],
              ),
              SwitchListTile(contentPadding: EdgeInsets.zero, title: Text(l.serverAutostart), subtitle: Text(l.serverAutostartHint), value: _autostart, onChanged: (value) => setState(() => _autostart = value)),
              SwitchListTile(contentPadding: EdgeInsets.zero, title: Text(l.serverAutoRestart), subtitle: Text(l.serverAutoRestartHint), value: _autoRestart, onChanged: (value) => setState(() => _autoRestart = value)),
              if (!providerInfo(kind: details.provider).isProxy)
                SwitchListTile(contentPadding: EdgeInsets.zero, title: Text(l.manageRcon), subtitle: Text(l.manageRconHint), value: _manageRcon, onChanged: (value) => setState(() => _manageRcon = value)),
            ],
          ),
        ),
        const SizedBox(height: 16),
        Align(
          alignment: Alignment.centerRight,
          child: FilledButton.icon(
            onPressed: _saving ? null : _save,
            icon: _saving ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.save_outlined),
            label: Text(l.save),
          ),
        ),
      ],
    );
  }
}
