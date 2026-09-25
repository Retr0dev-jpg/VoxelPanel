// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:math' as math;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/create/create_validation.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/provider_icon.dart';

/// Step-by-step creation of a new server; reports the new id through [onCreated].
class CreateWizard extends ConsumerStatefulWidget {
  const CreateWizard({super.key, required this.onCreated, required this.onBusyChanged});

  final ValueChanged<String> onCreated;
  final ValueChanged<bool> onBusyChanged;

  @override
  ConsumerState<CreateWizard> createState() => _CreateWizardState();
}

class _CreateWizardState extends ConsumerState<CreateWizard> {
  var _step = WizardStep.software;
  CreateIssue? _issue;
  var _busy = false;
  final _progress = <String>[];

  late final List<ProviderInfo> _providers;
  ProviderCategory? _category;
  ProviderInfo? _provider;
  final _name = TextEditingController();
  final _root = TextEditingController();

  var _snapshots = false;
  Future<List<VersionEntry>>? _versions;
  var _version = '';
  final _versionFilter = TextEditingController();
  Future<List<BuildEntry>>? _builds;
  var _build = '';
  Future<int>? _requiredJava;
  var _customJar = '';
  final _customVersion = TextEditingController();

  late final int _systemMb;
  var _ramMinMb = 2048;
  var _ramMaxMb = 4096;
  var _javaHome = '';
  late final List<JvmPresetInfo> _presets;
  var _preset = JvmPreset.aikar;
  final _flags = TextEditingController();

  var _autoPort = true;
  final _port = TextEditingController(text: '25565');
  final _motd = TextEditingController(text: 'A Minecraft Server');
  final _maxPlayers = TextEditingController(text: '20');
  final _seed = TextEditingController();
  var _gamemode = 'survival';
  var _difficulty = 'easy';
  var _onlineMode = true;
  var _eula = false;

  @override
  void initState() {
    super.initState();
    _providers = listProviders();
    _presets = jvmPresets();
    _systemMb = math.max(systemMemoryMb().toInt(), 1024);
    final suggestion = suggestRam();
    final settings = ref.read(settingsValueProvider);
    final defaults = settings?.defaults;
    _ramMinMb = parseMemoryMb(defaults?.ramMin ?? '') ?? parseMemoryMb(suggestion.ramMin) ?? 2048;
    _ramMaxMb = parseMemoryMb(defaults?.ramMax ?? '') ?? parseMemoryMb(suggestion.ramMax) ?? 4096;
    _ramMaxMb = _ramMaxMb.clamp(512, _systemMb);
    _ramMinMb = _ramMinMb.clamp(256, _ramMaxMb);
    _preset = defaults?.jvmPreset ?? JvmPreset.aikar;
    _flags.text = _presetFlags(_preset).join('\n');
    final preferred = defaults?.provider ?? 'paper';
    _selectProvider(_providers.where((info) => info.id == preferred).firstOrNull ?? _providers.firstOrNull, notify: false);
  }

  @override
  void dispose() {
    for (final controller in [_name, _root, _versionFilter, _customVersion, _flags, _port, _motd, _maxPlayers, _seed]) {
      controller.dispose();
    }
    super.dispose();
  }

  List<String> _presetFlags(JvmPreset preset) => _presets.where((info) => info.preset == preset).map((info) => info.flags).firstOrNull ?? const [];

  bool get _isCustom => _provider?.kind == ProviderKind.custom;
  bool get _isProxy => _provider?.isProxy ?? false;

  /// Replaced requests keep running; without a listener their failure crashes as unhandled.
  static Future<T> _observed<T>(Future<T> future) => future..ignore();

  void _selectProvider(ProviderInfo? provider, {bool notify = true}) {
    void apply() {
      _provider = provider;
      _version = '';
      _build = '';
      _builds = null;
      _requiredJava = null;
      _snapshots = false;
      _versions = provider == null || provider.kind == ProviderKind.custom
          ? null
          : _observed(listVersions(provider: provider.kind, includeSnapshots: false));
    }

    notify ? setState(apply) : apply();
  }

  void _selectVersion(String version) {
    final provider = _provider;
    if (provider == null) {
      return;
    }
    setState(() {
      _version = version;
      _build = '';
      _builds = provider.hasBuilds ? _observed(listBuilds(provider: provider.kind, version: version)) : null;
      _requiredJava = _observed(requiredJava(provider: provider.kind, version: version).then((value) => value));
    });
  }

  WizardInput get _input => WizardInput(
    name: _name.text,
    hasProvider: _provider != null,
    isCustom: _isCustom,
    customJar: _customJar,
    version: _version,
    ramMinMb: _ramMinMb,
    ramMaxMb: _ramMaxMb,
    port: _autoPort ? 0 : int.tryParse(_port.text) ?? -1,
    maxPlayers: int.tryParse(_maxPlayers.text) ?? 0,
    acceptEula: _eula,
    needsEula: !_isProxy,
  );

  void _next() {
    final issue = validateStep(_step, _input);
    setState(() => _issue = issue);
    if (issue == null && _step != WizardStep.summary) {
      setState(() => _step = WizardStep.values[_step.index + 1]);
    }
  }

  void _back() {
    if (_step.index > 0) {
      setState(() {
        _issue = null;
        _step = WizardStep.values[_step.index - 1];
      });
    }
  }

  void _setBusy(bool busy) {
    setState(() => _busy = busy);
    widget.onBusyChanged(busy);
  }

  Future<void> _create() async {
    final issue = validateAll(_input);
    setState(() => _issue = issue);
    if (issue != null) {
      return;
    }
    final provider = _provider!;
    final request = CreateServerRequest(
      name: _name.text.trim(),
      root: _root.text.trim(),
      provider: provider.kind,
      mcVersion: _isCustom ? _customVersion.text.trim() : _version,
      build: _build,
      jarPath: _isCustom ? _customJar : '',
      javaHome: _javaHome,
      ramMin: memoryValue(_ramMinMb),
      ramMax: memoryValue(_ramMaxMb),
      jvmFlags: _flags.text.split(RegExp(r'\s+')).where((flag) => flag.isNotEmpty).toList(),
      port: _autoPort ? 0 : int.parse(_port.text),
      motd: _motd.text.trim(),
      maxPlayers: int.tryParse(_maxPlayers.text) ?? 20,
      gamemode: _gamemode,
      difficulty: _difficulty,
      onlineMode: _onlineMode,
      levelSeed: _seed.text.trim(),
      acceptEula: _isProxy || _eula,
    );
    _setBusy(true);
    _progress.clear();
    String? created;
    try {
      await for (final event in createServer(request: request)) {
        if (!mounted) {
          return;
        }
        setState(() {
          final line = '${event.stage}: ${event.message}';
          if (_progress.isNotEmpty && event.fraction != null && !event.done && _progress.last.startsWith('${event.stage}: ')) {
            _progress[_progress.length - 1] = line;
          } else {
            _progress.add(line);
          }
        });
        if (event.error != null) {
          throw Exception(event.error);
        }
        created = event.serverId ?? created;
      }
    } catch (error) {
      if (mounted) {
        setState(() => _progress.add(describeError(context, error)));
      }
    } finally {
      if (mounted) {
        _setBusy(false);
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
    final titles = [l.stepSoftware, l.stepVersion, l.stepRuntime, l.stepSettings, l.stepSummary];
    return PanelCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _StepIndicator(titles: titles, current: _step.index, onTap: _busy ? null : (index) => index < _step.index ? setState(() => _step = WizardStep.values[index]) : null),
          const SizedBox(height: 20),
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 160),
            child: KeyedSubtree(
              key: ValueKey(_step),
              child: switch (_step) {
                WizardStep.software => _softwareStep(context),
                WizardStep.version => _versionStep(context),
                WizardStep.runtime => _runtimeStep(context),
                WizardStep.settings => _settingsStep(context),
                WizardStep.summary => _summaryStep(context),
              },
            ),
          ),
          if (_issue != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(createIssueText(l, _issue!), style: TextStyle(color: colors.danger)),
            ),
          const SizedBox(height: 16),
          Row(
            children: [
              if (_step.index > 0) OutlinedButton.icon(onPressed: _busy ? null : _back, icon: const Icon(Icons.arrow_back), label: Text(l.back)),
              const Spacer(),
              if (_step != WizardStep.summary)
                FilledButton.icon(onPressed: _busy ? null : _next, icon: const Icon(Icons.arrow_forward), label: Text(l.next))
              else
                FilledButton.icon(
                  onPressed: _busy ? null : _create,
                  icon: _busy ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.rocket_launch_outlined),
                  label: Text(_busy ? l.installing : l.createServer),
                ),
            ],
          ),
          if (_progress.isNotEmpty) ...[
            const SizedBox(height: 16),
            Container(
              constraints: const BoxConstraints(maxHeight: 260),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12)),
              child: SingleChildScrollView(
                reverse: true,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [for (final line in _progress) Text(line, style: const TextStyle(fontFamily: 'monospace', fontSize: 12, color: Color(0xFFDDDDDD)))],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _softwareStep(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final categories = {for (final provider in _providers) provider.category}.toList()..sort((a, b) => a.index.compareTo(b.index));
    final visible = _providers.where((provider) => _category == null || provider.category == _category).toList();
    final custom = providerInfo(kind: ProviderKind.custom);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(controller: _name, decoration: InputDecoration(labelText: l.serverName, hintText: l.serverNameHint, prefixIcon: const Icon(Icons.badge_outlined))),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(controller: _root, decoration: InputDecoration(labelText: l.destinationFolder, hintText: l.destinationFolderHint, prefixIcon: const Icon(Icons.folder_outlined))),
            ),
            const SizedBox(width: 8),
            OutlinedButton(
              onPressed: () async {
                final path = await FilePicker.getDirectoryPath(dialogTitle: l.serverFolder);
                if (path != null) {
                  setState(() => _root.text = path);
                }
              },
              child: Text(l.browse),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            ChoiceChip(label: Text(l.categoryAll), selected: _category == null, onSelected: (_) => setState(() => _category = null)),
            for (final category in categories)
              ChoiceChip(label: Text(categoryLabel(l, category)), selected: _category == category, onSelected: (_) => setState(() => _category = category)),
          ],
        ),
        const SizedBox(height: 12),
        ResponsiveGrid(
          minItemWidth: 250,
          children: [
            for (final provider in [...visible, if (_category == null) custom])
              _ProviderCard(provider: provider, selected: _provider?.kind == provider.kind, onTap: () => _selectProvider(provider)),
          ],
        ),
        if (_provider?.note.isNotEmpty ?? false) ...[
          const SizedBox(height: 12),
          Row(children: [Icon(Icons.info_outline, size: 16, color: colors.warning), const SizedBox(width: 8), Expanded(child: Text(_provider!.note))]),
        ],
      ],
    );
  }

  Widget _versionStep(BuildContext context) {
    final l = context.l10n;
    final provider = _provider!;
    if (_isCustom) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.upload_file),
            title: Text(_customJar.isEmpty ? l.noLocalJar : _customJar),
            trailing: FilledButton.tonal(
              onPressed: () async {
                final files = await FilePicker.pickFiles(dialogTitle: l.serverJarTitle, type: FileType.custom, allowedExtensions: const ['jar']);
                final path = files.isEmpty ? null : files.first.path;
                if (path != null) {
                  setState(() => _customJar = path);
                }
              },
              child: Text(l.chooseJar),
            ),
          ),
          const SizedBox(height: 8),
          TextField(controller: _customVersion, decoration: InputDecoration(labelText: l.customVersionLabel, helperText: l.customVersionHint)),
        ],
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _versionFilter,
                onChanged: (_) => setState(() {}),
                decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.searchVersion, isDense: true),
              ),
            ),
            if (provider.hasSnapshots) ...[
              const SizedBox(width: 12),
              FilterChip(
                label: Text(l.showSnapshots),
                selected: _snapshots,
                onSelected: (value) => setState(() {
                  _snapshots = value;
                  _versions = _observed(listVersions(provider: provider.kind, includeSnapshots: value));
                }),
              ),
            ],
          ],
        ),
        const SizedBox(height: 12),
        SizedBox(
          height: 240,
          child: FutureBuilder<List<VersionEntry>>(
            future: _versions,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                return ErrorState(error: snapshot.error!, onRetry: () => setState(() => _versions = _observed(listVersions(provider: provider.kind, includeSnapshots: _snapshots))));
              }
              final versions = snapshot.data;
              if (versions == null) {
                return const Center(child: CircularProgressIndicator());
              }
              final filter = _versionFilter.text.trim();
              final visible = versions.where((entry) => filter.isEmpty || entry.id.contains(filter)).toList();
              if (_version.isEmpty && versions.isNotEmpty) {
                WidgetsBinding.instance.addPostFrameCallback((_) {
                  if (mounted && _version.isEmpty) {
                    _selectVersion(versions.firstWhere((entry) => entry.stable, orElse: () => versions.first).id);
                  }
                });
              }
              return SingleChildScrollView(
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final (index, entry) in visible.indexed)
                      ChoiceChip(
                        label: Text(index == 0 && filter.isEmpty && entry.stable ? l.latestVersion(entry.id) : entry.id),
                        avatar: entry.stable ? null : const Icon(Icons.science_outlined, size: 16),
                        selected: _version == entry.id,
                        onSelected: (_) => _selectVersion(entry.id),
                      ),
                  ],
                ),
              );
            },
          ),
        ),
        if (provider.hasBuilds && _builds != null) ...[
          const SizedBox(height: 12),
          FutureBuilder<List<BuildEntry>>(
            future: _builds,
            builder: (context, snapshot) {
              final builds = snapshot.data ?? const <BuildEntry>[];
              return DropdownButtonFormField<String>(
                key: ValueKey('builds-$_version-${builds.length}'),
                initialValue: _build,
                isExpanded: true,
                decoration: InputDecoration(labelText: l.buildLabel, helperText: snapshot.connectionState == ConnectionState.waiting ? l.loading : null),
                items: [
                  DropdownMenuItem(value: '', child: Text(l.latestBuild)),
                  for (final build in builds) DropdownMenuItem(value: build.id, child: Text(build.label)),
                ],
                onChanged: (value) => setState(() => _build = value ?? ''),
              );
            },
          ),
        ],
        if (_requiredJava != null) ...[
          const SizedBox(height: 12),
          FutureBuilder<int>(
            future: _requiredJava,
            builder: (context, snapshot) => Row(
              children: [
                const Icon(Icons.coffee_outlined, size: 16),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(
                    snapshot.hasError
                        ? describeError(context, snapshot.error!)
                        : snapshot.hasData
                        ? l.requiresJava(snapshot.data!)
                        : l.loading,
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _runtimeStep(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final maxMb = _systemMb;
    final step = 256;
    double snap(double value) => (value / step).round() * step.toDouble();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        FutureBuilder<List<JavaRuntimeInfo>>(
          future: listRuntimes(),
          builder: (context, snapshot) {
            final runtimes = snapshot.data ?? const <JavaRuntimeInfo>[];
            return DropdownButtonFormField<String>(
              key: ValueKey('java-${runtimes.length}'),
              initialValue: runtimes.any((runtime) => runtime.path == _javaHome) ? _javaHome : '',
              isExpanded: true,
              decoration: InputDecoration(labelText: l.javaRuntime, helperText: l.javaAutoHint),
              items: [
                DropdownMenuItem(value: '', child: Text(l.javaAutomatic)),
                for (final runtime in runtimes)
                  DropdownMenuItem(value: runtime.path, child: Text(runtime.major == 0 ? runtime.name : 'Java ${runtime.major} · ${runtime.name}', overflow: TextOverflow.ellipsis)),
              ],
              onChanged: (value) => setState(() => _javaHome = value ?? ''),
            );
          },
        ),
        const SizedBox(height: 20),
        Text(l.memoryRam, style: const TextStyle(fontWeight: FontWeight.w600)),
        Text(l.systemMemory(formatBytes(maxMb * 1024 * 1024)), style: TextStyle(color: colors.muted, fontSize: 12)),
        const SizedBox(height: 8),
        Row(
          children: [
            SizedBox(width: 110, child: Text('${l.ramMax}: ${memoryValue(_ramMaxMb)}')),
            Expanded(
              child: Slider(
                value: _ramMaxMb.toDouble().clamp(512, maxMb.toDouble()),
                min: 512,
                max: maxMb.toDouble(),
                divisions: math.max(1, (maxMb - 512) ~/ step),
                label: memoryValue(_ramMaxMb),
                onChanged: (value) => setState(() {
                  _ramMaxMb = snap(value).toInt().clamp(512, maxMb);
                  _ramMinMb = math.min(_ramMinMb, _ramMaxMb);
                }),
              ),
            ),
          ],
        ),
        Row(
          children: [
            SizedBox(width: 110, child: Text('${l.ramMin}: ${memoryValue(_ramMinMb)}')),
            Expanded(
              child: Slider(
                value: _ramMinMb.toDouble().clamp(256, _ramMaxMb.toDouble()),
                min: 256,
                max: math.max(_ramMaxMb.toDouble(), 257),
                divisions: math.max(1, (_ramMaxMb - 256) ~/ step),
                label: memoryValue(_ramMinMb),
                onChanged: (value) => setState(() => _ramMinMb = snap(value).toInt().clamp(256, _ramMaxMb)),
              ),
            ),
          ],
        ),
        if (_ramMaxMb > maxMb * 0.8) Text(l.ramWarning, style: TextStyle(color: colors.warning, fontSize: 12)),
        const SizedBox(height: 16),
        DropdownButtonFormField<JvmPreset>(
          initialValue: _preset,
          decoration: InputDecoration(labelText: l.settingJvmPreset),
          items: [
            DropdownMenuItem(value: JvmPreset.aikar, child: Text(l.jvmPresetAikar)),
            DropdownMenuItem(value: JvmPreset.g1, child: Text(l.jvmPresetG1)),
            DropdownMenuItem(value: JvmPreset.zgc, child: Text(l.jvmPresetZgc)),
            DropdownMenuItem(value: JvmPreset.none, child: Text(l.jvmPresetNone)),
          ],
          onChanged: (value) => setState(() {
            _preset = value ?? _preset;
            _flags.text = _presetFlags(_preset).join('\n');
          }),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _flags,
          minLines: 3,
          maxLines: 6,
          style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
          decoration: InputDecoration(labelText: l.jvmFlags, helperText: l.jvmFlagsHint),
        ),
      ],
    );
  }

  Widget _settingsStep(BuildContext context) {
    final l = context.l10n;
    final digits = [FilteringTextInputFormatter.digitsOnly];
    if (_isProxy) {
      return Text(l.proxySettingsHint);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(l.autoPort),
                subtitle: Text(l.autoPortHint),
                value: _autoPort,
                onChanged: (value) => setState(() => _autoPort = value),
              ),
            ),
            const SizedBox(width: 12),
            SizedBox(
              width: 140,
              child: TextField(controller: _port, enabled: !_autoPort, inputFormatters: digits, decoration: InputDecoration(labelText: l.propPort)),
            ),
          ],
        ),
        const SizedBox(height: 12),
        TextField(controller: _motd, maxLength: 256, decoration: InputDecoration(labelText: l.propMotd)),
        Row(
          children: [
            Expanded(child: TextField(controller: _maxPlayers, inputFormatters: digits, decoration: InputDecoration(labelText: l.propMaxPlayers))),
            const SizedBox(width: 12),
            Expanded(child: TextField(controller: _seed, decoration: InputDecoration(labelText: l.propLevelSeed, hintText: l.seedHint))),
          ],
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: DropdownButtonFormField<String>(
                initialValue: _gamemode,
                decoration: InputDecoration(labelText: l.propGamemode),
                items: [
                  DropdownMenuItem(value: 'survival', child: Text(l.gamemodeSurvival)),
                  DropdownMenuItem(value: 'creative', child: Text(l.gamemodeCreative)),
                  DropdownMenuItem(value: 'adventure', child: Text(l.gamemodeAdventure)),
                  DropdownMenuItem(value: 'spectator', child: Text(l.gamemodeSpectator)),
                ],
                onChanged: (value) => setState(() => _gamemode = value ?? _gamemode),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: DropdownButtonFormField<String>(
                initialValue: _difficulty,
                decoration: InputDecoration(labelText: l.propDifficulty),
                items: [
                  DropdownMenuItem(value: 'peaceful', child: Text(l.difficultyPeaceful)),
                  DropdownMenuItem(value: 'easy', child: Text(l.difficultyEasy)),
                  DropdownMenuItem(value: 'normal', child: Text(l.difficultyNormal)),
                  DropdownMenuItem(value: 'hard', child: Text(l.difficultyHard)),
                ],
                onChanged: (value) => setState(() => _difficulty = value ?? _difficulty),
              ),
            ),
          ],
        ),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: Text(l.propOnlineMode),
          subtitle: Text(l.onlineModeHint),
          value: _onlineMode,
          onChanged: (value) => setState(() => _onlineMode = value),
        ),
      ],
    );
  }

  Widget _summaryStep(BuildContext context) {
    final l = context.l10n;
    final provider = _provider!;
    Widget row(String label, String value) => Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 180, child: Text(label, style: TextStyle(color: context.voxel.muted))),
          Expanded(child: SelectableText(value)),
        ],
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            ProviderIcon(provider.kind, size: 48),
            const SizedBox(width: 12),
            Expanded(child: Text(_name.text.trim(), style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700))),
          ],
        ),
        const SizedBox(height: 12),
        row(l.stepSoftware, provider.name),
        row(l.stepVersion, _isCustom ? (_customVersion.text.trim().isEmpty ? _customJar : '${_customVersion.text.trim()} · $_customJar') : '$_version${_build.isEmpty ? '' : ' · #$_build'}'),
        row(l.destinationFolder, _root.text.trim().isEmpty ? l.destinationFolderHint : _root.text.trim()),
        row(l.memoryRam, '${memoryValue(_ramMinMb)} – ${memoryValue(_ramMaxMb)}'),
        row(l.javaRuntime, _javaHome.isEmpty ? l.javaAutomatic : _javaHome),
        if (!_isProxy) ...[
          row(l.propPort, _autoPort ? l.autoPort : _port.text),
          row(l.propMotd, _motd.text),
          row(l.propGamemode, _gamemode),
        ],
        if (!_isProxy)
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: _eula,
            onChanged: _busy ? null : (value) => setState(() => _eula = value ?? false),
            title: Text(l.eulaCheckbox),
            subtitle: Text(l.eulaHint),
          ),
      ],
    );
  }
}

class _ProviderCard extends StatelessWidget {
  const _ProviderCard({required this.provider, required this.selected, required this.onTap});

  final ProviderInfo provider;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    final l = context.l10n;
    return Material(
      color: selected ? colors.accent.withValues(alpha: 0.16) : colors.field,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14), side: BorderSide(color: selected ? colors.accent : colors.cardBorder, width: selected ? 2 : 1)),
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              ProviderIcon(provider.kind, size: 40),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(child: Text(provider.name, style: const TextStyle(fontWeight: FontWeight.w700), overflow: TextOverflow.ellipsis)),
                        if (provider.deprecated) ...[const SizedBox(width: 6), Icon(Icons.warning_amber, size: 14, color: colors.warning)],
                      ],
                    ),
                    Text(categoryLabel(l, provider.category), style: TextStyle(color: colors.accent, fontSize: 11)),
                    const SizedBox(height: 4),
                    Text(provider.description, maxLines: 3, overflow: TextOverflow.ellipsis, style: TextStyle(color: colors.muted, fontSize: 12)),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StepIndicator extends StatelessWidget {
  const _StepIndicator({required this.titles, required this.current, required this.onTap});

  final List<String> titles;
  final int current;
  final ValueChanged<int>? onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    return Wrap(
      spacing: 6,
      runSpacing: 6,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        for (var index = 0; index < titles.length; index++) ...[
          InkWell(
            borderRadius: BorderRadius.circular(20),
            onTap: onTap == null ? null : () => onTap!(index),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  CircleAvatar(
                    radius: 12,
                    backgroundColor: index <= current ? colors.accent : colors.track,
                    child: index < current
                        ? const Icon(Icons.check, size: 14, color: Colors.white)
                        : Text('${index + 1}', style: TextStyle(fontSize: 12, color: index == current ? Colors.white : colors.muted)),
                  ),
                  const SizedBox(width: 6),
                  Text(titles[index], style: TextStyle(fontWeight: index == current ? FontWeight.w700 : FontWeight.w400, color: index == current ? null : colors.muted)),
                ],
              ),
            ),
          ),
          if (index < titles.length - 1) Icon(Icons.chevron_right, size: 16, color: colors.muted),
        ],
      ],
    );
  }
}
