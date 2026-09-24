// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

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

String? validateCreate(CreateInput input) {
  if (input.name.trim().isEmpty) {
    return 'Inserisci un nome.';
  }
  if (!input.acceptEula) {
    return "Accetta l'EULA di Minecraft per continuare.";
  }
  if (input.automatic && input.paperVersion.isEmpty) {
    return 'Seleziona una versione Paper.';
  }
  if (!input.automatic && input.jarPath.isEmpty && input.paperVersion.isEmpty) {
    return 'Seleziona una versione Paper oppure un jar.';
  }
  if (!input.automatic && input.javaHome.isEmpty && input.javaMajor == 0) {
    return 'Seleziona un runtime Java.';
  }
  return null;
}

class CreateWizardBody extends StatefulWidget {
  const CreateWizardBody({
    super.key,
    required this.paperVersions,
    required this.ramChoices,
    required this.jvmFlags,
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
  final List<JvmFlagChoice> jvmFlags;
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
  var _error = '';
  final _flags = <String>{};

  @override
  void initState() {
    super.initState();
    _ramMin = widget.suggestedMin;
    _ramMax = widget.suggestedMax;
    for (final flag in widget.jvmFlags) {
      if (flag.recommended) {
        _flags.add(flag.flag);
      }
    }
  }

  @override
  void dispose() {
    _name.dispose();
    _root.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        TextField(
          controller: _name,
          decoration: const InputDecoration(labelText: 'Nome del server', border: OutlineInputBorder()),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _root,
          decoration: const InputDecoration(
            labelText: 'Cartella',
            hintText: 'Vuota: VoxelPanel ne crea una nei dati locali',
            border: OutlineInputBorder(),
          ),
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton(
            onPressed: widget.busy
                ? null
                : () async {
                    final path = await widget.pickDirectory();
                    if (path != null) {
                      setState(() => _root.text = path);
                    }
                  },
            child: const Text('Sfoglia'),
          ),
        ),
        SegmentedButton<bool>(
          segments: const [
            ButtonSegment(value: true, label: Text('Automatica')),
            ButtonSegment(value: false, label: Text('Manuale')),
          ],
          selected: {_automatic},
          onSelectionChanged: widget.busy
              ? null
              : (value) => setState(() => _automatic = value.first),
        ),
        const SizedBox(height: 16),
        if (_automatic) _paperDropdown() else ..._manualFields(),
        const SizedBox(height: 12),
        CheckboxListTile(
          contentPadding: EdgeInsets.zero,
          value: _eula,
          onChanged: widget.busy ? null : (value) => setState(() => _eula = value ?? false),
          title: const Text("Accetto l'EULA di Minecraft (eula=true)"),
        ),
        if (_error.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: Text(_error, style: TextStyle(color: Theme.of(context).colorScheme.error)),
          ),
        FilledButton(
          onPressed: widget.busy ? null : _submit,
          child: Text(widget.busy ? 'Installazione...' : 'Crea server'),
        ),
        if (widget.progress.isNotEmpty) ...[
          const SizedBox(height: 16),
          for (final line in widget.progress) Text(line),
        ],
      ],
    );
  }

  Widget _paperDropdown() {
    return DropdownButtonFormField<String>(
      initialValue: _paper.isEmpty ? null : _paper,
      decoration: const InputDecoration(labelText: 'Versione Paper', border: OutlineInputBorder()),
      items: [
        for (final version in widget.paperVersions)
          DropdownMenuItem(value: version, child: Text(version)),
      ],
      onChanged: widget.busy ? null : (value) => setState(() => _paper = value ?? ''),
    );
  }

  List<Widget> _manualFields() {
    return [
      DropdownButtonFormField<String>(
        initialValue: _javaHome.isEmpty ? null : _javaHome,
        decoration: const InputDecoration(labelText: 'Runtime Java installato', border: OutlineInputBorder()),
        items: [
          for (final runtime in widget.runtimes)
            DropdownMenuItem(
              value: runtime.path,
              child: Text(runtime.major == 0 ? runtime.name : 'Java ${runtime.major}'),
            ),
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
        child: TextButton(
          onPressed: widget.busy || widget.javaReleases.isEmpty ? null : _pickJavaRelease,
          child: const Text('Scarica un altro Java'),
        ),
      ),
      _paperDropdown(),
      const SizedBox(height: 12),
      ListTile(
        contentPadding: EdgeInsets.zero,
        title: Text(_jar.isEmpty ? 'Nessun jar locale' : _jar),
        trailing: TextButton(
          onPressed: widget.busy
              ? null
              : () async {
                  final path = await widget.pickJar();
                  if (path != null) {
                    setState(() => _jar = path);
                  }
                },
          child: const Text('Scegli jar'),
        ),
      ),
      _ramDropdown('RAM minima', _ramMin, (value) => _ramMin = value),
      const SizedBox(height: 12),
      _ramDropdown('RAM massima', _ramMax, (value) => _ramMax = value),
      const SizedBox(height: 12),
      const Text('Flag JVM'),
      for (final flag in widget.jvmFlags)
        CheckboxListTile(
          contentPadding: EdgeInsets.zero,
          value: _flags.contains(flag.flag),
          title: Text(flag.flag),
          onChanged: widget.busy
              ? null
              : (checked) => setState(() {
                  if (checked ?? false) {
                    _flags.add(flag.flag);
                  } else {
                    _flags.remove(flag.flag);
                  }
                }),
        ),
    ];
  }

  Widget _ramDropdown(String label, String current, void Function(String value) assign) {
    return DropdownButtonFormField<String>(
      initialValue: widget.ramChoices.any((choice) => choice.value == current) ? current : null,
      decoration: InputDecoration(labelText: label, border: const OutlineInputBorder()),
      items: [
        for (final choice in widget.ramChoices)
          DropdownMenuItem(value: choice.value, child: Text('${choice.label} (${choice.value})')),
      ],
      onChanged: widget.busy ? null : (value) => setState(() => assign(value ?? current)),
    );
  }

  Future<void> _pickJavaRelease() async {
    final major = await showDialog<int>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Versione Java'),
        children: [
          for (final release in widget.javaReleases)
            SimpleDialogOption(
              onPressed: () => Navigator.pop(context, release.major),
              child: Text('Java ${release.major}${release.lts ? ' LTS' : ''}'),
            ),
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
    final error = validateCreate(
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
    if (error != null) {
      setState(() => _error = error);
      return;
    }
    setState(() => _error = '');
    if (_automatic) {
      await widget.onAuto(
        AutoInstallRequest(
          name: _name.text.trim(),
          root: _root.text.trim(),
          paperVersion: _paper,
          acceptEula: true,
          ramMin: _ramMin,
          ramMax: _ramMax,
        ),
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
        jvmFlags: _flags.toList(),
        acceptEula: true,
      ),
    );
  }
}
