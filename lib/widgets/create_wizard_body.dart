// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

class CreateInput {
  const CreateInput({
    required this.name,
    required this.acceptEula,
    required this.automatic,
    required this.paperVersion,
    required this.jarPath,
    required this.javaHome,
    required this.javaMajor,
  });

  final String name;
  final bool acceptEula;
  final bool automatic;
  final String paperVersion;
  final String jarPath;
  final String javaHome;
  final int javaMajor;
}

enum CreateIssue { missingName, eulaNotAccepted, missingVersion, missingVersionOrJar, missingJava }

CreateIssue? validateCreate(CreateInput input) {
  if (input.name.trim().isEmpty) {
    return CreateIssue.missingName;
  }
  if (!input.acceptEula) {
    return CreateIssue.eulaNotAccepted;
  }
  if (input.automatic && input.paperVersion.isEmpty) {
    return CreateIssue.missingVersion;
  }
  if (!input.automatic && input.jarPath.isEmpty && input.paperVersion.isEmpty) {
    return CreateIssue.missingVersionOrJar;
  }
  if (!input.automatic && input.javaHome.isEmpty && input.javaMajor == 0) {
    return CreateIssue.missingJava;
  }
  return null;
}

String createIssueText(AppLocalizations l, CreateIssue issue) => switch (issue) {
  CreateIssue.missingName => l.issueMissingName,
  CreateIssue.eulaNotAccepted => l.issueEula,
  CreateIssue.missingVersion => l.issueMissingVersion,
  CreateIssue.missingVersionOrJar => l.issueMissingVersionOrJar,
  CreateIssue.missingJava => l.issueMissingJava,
};

class CreateWizardBody extends StatefulWidget {
  const CreateWizardBody({
    super.key,
    required this.paperVersions,
    required this.ramChoices,
    required this.jvmPresets,
    required this.defaultPreset,
    required this.runtimes,
    required this.javaReleases,
    required this.suggestedMin,
    required this.suggestedMax,
    required this.progress,
    required this.busy,
    required this.onAuto,
    required this.onManual,
    required this.pickDirectory,
    required this.pickJar,
    required this.installJava,
  });

  final List<String> paperVersions;
  final List<RamChoice> ramChoices;
  final List<JvmPresetInfo> jvmPresets;
  final JvmPreset defaultPreset;
  final List<JavaRuntimeInfo> runtimes;
  final List<JavaReleaseInfo> javaReleases;
  final String suggestedMin;
  final String suggestedMax;
  final List<String> progress;
  final bool busy;
  final Future<void> Function(AutoInstallRequest request) onAuto;
  final Future<void> Function(ManualInstallRequest request) onManual;
  final Future<String?> Function() pickDirectory;
  final Future<String?> Function() pickJar;
  final Future<void> Function(int major) installJava;

  @override
  State<CreateWizardBody> createState() => _CreateWizardBodyState();
}

class _CreateWizardBodyState extends State<CreateWizardBody> {
  final _name = TextEditingController();
  final _root = TextEditingController();
  var _automatic = true;
  var _paper = '';
  var _jar = '';
  var _javaHome = '';
  var _javaMajor = 0;
  var _ramMin = '';
  var _ramMax = '';
  var _eula = false;
  CreateIssue? _issue;
  late JvmPreset _preset = widget.defaultPreset;
  late final _flags = TextEditingController(text: _presetFlags(widget.defaultPreset).join('\n'));

  List<String> _presetFlags(JvmPreset preset) => widget.jvmPresets.where((info) => info.preset == preset).map((info) => info.flags).firstOrNull ?? const [];

  @override
  void initState() {
    super.initState();
    _ramMin = widget.suggestedMin;
    _ramMax = widget.suggestedMax;
    _paper = widget.paperVersions.isEmpty ? '' : widget.paperVersions.first;
  }

  @override
  void dispose() {
    _name.dispose();
    _root.dispose();
    _flags.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return PanelCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.serverName, style: const TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          TextField(controller: _name, decoration: InputDecoration(prefixIcon: const Icon(Icons.view_in_ar), hintText: l.serverNameHint)),
          const SizedBox(height: 16),
          Text(l.destinationFolder, style: const TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: TextField(controller: _root, decoration: InputDecoration(prefixIcon: const Icon(Icons.folder_outlined), hintText: l.destinationFolderHint)),
              ),
              const SizedBox(width: 8),
              OutlinedButton.icon(
                onPressed: widget.busy
                    ? null
                    : () async {
                        final path = await widget.pickDirectory();
                        if (path != null) {
                          setState(() => _root.text = path);
                        }
                      },
                icon: const Icon(Icons.folder_open_outlined),
                label: Text(l.browse),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Text(l.installMethod, style: const TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          SegmentedButton<bool>(
            segments: [
              ButtonSegment(value: true, icon: const Icon(Icons.auto_awesome), label: Text(l.methodAutomatic)),
              ButtonSegment(value: false, icon: const Icon(Icons.tune), label: Text(l.methodManual)),
            ],
            selected: {_automatic},
            onSelectionChanged: widget.busy ? null : (value) => setState(() => _automatic = value.first),
          ),
          const SizedBox(height: 16),
          if (_automatic) ...[
            _paperDropdown(context),
            const SizedBox(height: 16),
            Text(l.memoryRam, style: const TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final choice in widget.ramChoices)
                  ChoiceChip(
                    label: Text(choice.label),
                    selected: _ramMax == choice.value,
                    onSelected: widget.busy ? null : (_) => setState(() => _ramMax = choice.value),
                  ),
              ],
            ),
          ] else
            ..._manualFields(context),
          const SizedBox(height: 8),
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: _eula,
            onChanged: widget.busy ? null : (value) => setState(() => _eula = value ?? false),
            title: Text(l.eulaCheckbox),
            subtitle: Text(l.eulaHint),
          ),
          if (_issue != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Text(createIssueText(l, _issue!), style: TextStyle(color: colors.danger)),
            ),
          FilledButton.icon(
            onPressed: widget.busy ? null : _submit,
            icon: widget.busy ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)) : const Icon(Icons.play_arrow),
            label: Text(widget.busy ? l.installing : l.createServer),
          ),
          if (widget.progress.isNotEmpty) ...[
            const SizedBox(height: 16),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12)),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [for (final line in widget.progress) Text(line, style: const TextStyle(fontFamily: 'monospace', fontSize: 12, color: Color(0xFFDDDDDD)))],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _paperDropdown(BuildContext context) {
    final l = context.l10n;
    final latest = widget.paperVersions.isEmpty ? null : widget.paperVersions.first;
    return DropdownButtonFormField<String>(
      initialValue: _paper.isEmpty ? null : _paper,
      decoration: InputDecoration(labelText: l.paperVersion),
      items: [
        for (final version in widget.paperVersions) DropdownMenuItem(value: version, child: Text(version == latest ? l.latestVersion(version) : version)),
      ],
      onChanged: widget.busy ? null : (value) => setState(() => _paper = value ?? ''),
    );
  }

  List<Widget> _manualFields(BuildContext context) {
    final l = context.l10n;
    return [
      DropdownButtonFormField<String>(
        initialValue: _javaHome.isEmpty ? null : _javaHome,
        decoration: InputDecoration(labelText: l.installedJava),
        items: [
          for (final runtime in widget.runtimes)
            DropdownMenuItem(value: runtime.path, child: Text(runtime.major == 0 ? runtime.name : 'Java ${runtime.major}')),
        ],
        onChanged: widget.busy
            ? null
            : (value) => setState(() {
                _javaHome = value ?? '';
                _javaMajor = 0;
              }),
      ),
      Align(
        alignment: Alignment.centerLeft,
        child: TextButton(onPressed: widget.busy || widget.javaReleases.isEmpty ? null : _pickJavaRelease, child: Text(l.downloadOtherJava)),
      ),
      _paperDropdown(context),
      const SizedBox(height: 12),
      ListTile(
        contentPadding: EdgeInsets.zero,
        title: Text(_jar.isEmpty ? l.noLocalJar : _jar),
        trailing: TextButton(
          onPressed: widget.busy
              ? null
              : () async {
                  final path = await widget.pickJar();
                  if (path != null) {
                    setState(() => _jar = path);
                  }
                },
          child: Text(l.chooseJar),
        ),
      ),
      _ramDropdown(l.ramMin, _ramMin, (value) => _ramMin = value),
      const SizedBox(height: 12),
      _ramDropdown(l.ramMax, _ramMax, (value) => _ramMax = value),
      const SizedBox(height: 12),
      DropdownButtonFormField<JvmPreset>(
        initialValue: _preset,
        decoration: InputDecoration(labelText: l.settingJvmPreset),
        items: [
          DropdownMenuItem(value: JvmPreset.aikar, child: Text(l.jvmPresetAikar)),
          DropdownMenuItem(value: JvmPreset.g1, child: Text(l.jvmPresetG1)),
          DropdownMenuItem(value: JvmPreset.zgc, child: Text(l.jvmPresetZgc)),
          DropdownMenuItem(value: JvmPreset.none, child: Text(l.jvmPresetNone)),
        ],
        onChanged: widget.busy
            ? null
            : (value) => setState(() {
                _preset = value ?? _preset;
                _flags.text = _presetFlags(_preset).join('\n');
              }),
      ),
      const SizedBox(height: 12),
      TextField(
        controller: _flags,
        enabled: !widget.busy,
        minLines: 3,
        maxLines: 8,
        style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
        decoration: InputDecoration(labelText: l.jvmFlags, helperText: l.jvmFlagsHint),
      ),
    ];
  }

  Widget _ramDropdown(String label, String current, void Function(String value) assign) {
    return DropdownButtonFormField<String>(
      initialValue: widget.ramChoices.any((choice) => choice.value == current) ? current : null,
      decoration: InputDecoration(labelText: label),
      items: [for (final choice in widget.ramChoices) DropdownMenuItem(value: choice.value, child: Text('${choice.label} (${choice.value})'))],
      onChanged: widget.busy ? null : (value) => setState(() => assign(value ?? current)),
    );
  }

  Future<void> _pickJavaRelease() async {
    final l = context.l10n;
    final major = await showDialog<int>(
      context: context,
      builder: (context) => SimpleDialog(
        title: Text(l.javaVersion),
        children: [
          for (final release in widget.javaReleases)
            SimpleDialogOption(onPressed: () => Navigator.pop(context, release.major), child: Text('Java ${release.major}${release.lts ? ' LTS' : ''}')),
        ],
      ),
    );
    if (major != null) {
      await widget.installJava(major);
      if (mounted) {
        setState(() {
          _javaMajor = major;
          _javaHome = '';
        });
      }
    }
  }

  Future<void> _submit() async {
    final issue = validateCreate(
      CreateInput(
        name: _name.text,
        acceptEula: _eula,
        automatic: _automatic,
        paperVersion: _paper,
        jarPath: _jar,
        javaHome: _javaHome,
        javaMajor: _javaMajor,
      ),
    );
    setState(() => _issue = issue);
    if (issue != null) {
      return;
    }
    if (_automatic) {
      await widget.onAuto(
        AutoInstallRequest(name: _name.text.trim(), root: _root.text.trim(), paperVersion: _paper, acceptEula: true, ramMin: _ramMin, ramMax: _ramMax),
      );
      return;
    }
    await widget.onManual(
      ManualInstallRequest(
        name: _name.text.trim(),
        root: _root.text.trim(),
        paperVersion: _paper,
        jarPath: _jar,
        javaMajor: _javaMajor,
        javaHome: _javaHome,
        ramMin: _ramMin,
        ramMax: _ramMax,
        jvmFlags: _flags.text.split(RegExp(r'\s+')).where((flag) => flag.isNotEmpty).toList(),
        acceptEula: true,
      ),
    );
  }
}
