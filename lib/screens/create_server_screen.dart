// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/create_wizard_body.dart';

class CreateServerScreen extends StatefulWidget {
  const CreateServerScreen({super.key, this.startOnImport = false});

  final bool startOnImport;

  @override
  State<CreateServerScreen> createState() => _CreateServerScreenState();
}

class _CreateServerScreenState extends State<CreateServerScreen> {
  List<String> _versions = [];
  List<RamChoice> _ram = [];
  List<JvmFlagChoice> _flags = [];
  List<JavaRuntimeInfo> _runtimes = [];
  List<JavaReleaseInfo> _releases = [];
  var _min = '2G';
  var _max = '4G';
  var _ready = false;
  var _busy = false;
  var _importing = false;
  var _error = '';
  final _progress = <String>[];
  final _importPath = TextEditingController();
  ImportPreview? _preview;
  var _acceptEula = false;

  @override
  void initState() {
    super.initState();
    _importing = widget.startOnImport;
    _load();
  }

  @override
  void dispose() {
    _importPath.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final versions = await listPaperVersions();
      final releases = await listJavaReleases();
      final runtimes = await listRuntimes();
      final suggestion = suggestRam();
      if (!mounted) {
        return;
      }
      setState(() {
        _versions = versions;
        _releases = releases;
        _runtimes = runtimes;
        _ram = ramPresets();
        _flags = jvmFlagChoices();
        _min = suggestion.ramMin;
        _max = suggestion.ramMax;
        _ready = true;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = readableError(error));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            entries: const [SidebarEntry(label: 'Server', icon: Icons.dns_outlined)],
            selected: 0,
            onSelected: (_) {},
          ),
          Expanded(
            child: !_ready
                ? Center(child: _error.isEmpty ? const CircularProgressIndicator() : Text(_error))
                : ListView(
                    padding: const EdgeInsets.fromLTRB(28, 18, 28, 28),
                    children: [
                      Align(
                        alignment: Alignment.centerLeft,
                        child: IconButton(onPressed: () => Navigator.pop(context), icon: const Icon(Icons.arrow_back)),
                      ),
                      const Text('Nuovo server', style: TextStyle(fontSize: 32, fontWeight: FontWeight.w700)),
                      const SizedBox(height: 4),
                      const Text('Crea un nuovo server Paper o importa una cartella già esistente.', style: TextStyle(color: panelMuted)),
                      const SizedBox(height: 20),
                      Row(
                        children: [
                          _ModeCard(
                            selected: !_importing,
                            icon: Icons.view_in_ar,
                            title: 'Crea server',
                            subtitle: 'Configura e avvia un nuovo server',
                            onTap: _busy ? null : () => setState(() => _importing = false),
                          ),
                          const SizedBox(width: 12),
                          _ModeCard(
                            selected: _importing,
                            icon: Icons.download_outlined,
                            title: 'Importa server',
                            subtitle: 'Importa da una cartella esistente',
                            onTap: _busy ? null : () => setState(() => _importing = true),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      if (_importing)
                        _importForm()
                      else
                        CreateWizardBody(
                          paperVersions: _versions,
                          ramChoices: _ram,
                          jvmFlags: _flags,
                          runtimes: _runtimes,
                          javaReleases: _releases,
                          suggestedMin: _min,
                          suggestedMax: _max,
                          progress: _progress,
                          busy: _busy,
                          onAuto: (request) => _install(installAuto(request: request)),
                          onManual: (request) => _install(installManual(request: request)),
                          pickDirectory: () => FilePicker.getDirectoryPath(dialogTitle: 'Cartella del server'),
                          pickJar: _pickJar,
                          installJava: _installJava,
                        ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  Widget _importForm() {
    final preview = _preview;
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: panelCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: panelCardBorder),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('Cartella del server', style: TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(child: TextField(controller: _importPath, decoration: const InputDecoration(prefixIcon: Icon(Icons.folder_outlined)))),
              const SizedBox(width: 8),
              OutlinedButton(onPressed: _busy ? null : _browseImport, child: const Text('Sfoglia')),
            ],
          ),
          if (preview != null) ...[
            const SizedBox(height: 12),
            Text('Paper: ${preview.paperVersion.isEmpty ? 'non rilevato' : preview.paperVersion}'),
            Text('Java: ${preview.javaMajor == 0 ? 'non rilevato' : preview.javaMajor}'),
            Text('RAM: ${preview.ramMin} / ${preview.ramMax}'),
            Text('Plugin: ${preview.pluginCount} · Mondi: ${preview.worldCount}'),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              value: _acceptEula,
              onChanged: (value) => setState(() => _acceptEula = value ?? false),
              title: const Text("Accetto l'EULA di Minecraft (eula=true)"),
            ),
          ],
          const SizedBox(height: 12),
          FilledButton(onPressed: _busy || preview == null ? null : _confirmImport, child: const Text('Importa server')),
        ],
      ),
    );
  }

  Future<void> _browseImport() async {
    final path = await FilePicker.getDirectoryPath(dialogTitle: 'Cartella del server');
    if (path == null) {
      return;
    }
    try {
      final preview = await previewImport(path: path);
      if (!mounted) {
        return;
      }
      setState(() {
        _importPath.text = path;
        _preview = preview;
        _acceptEula = preview.hasEula;
      });
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }

  Future<void> _confirmImport() async {
    final preview = _preview;
    if (preview == null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await importServer(path: preview.root, name: preview.name, acceptEula: _acceptEula);
      if (mounted) {
        Navigator.pop(context, true);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<String?> _pickJar() async {
    final files = await FilePicker.pickFiles(
      dialogTitle: 'Jar Paper',
      type: FileType.custom,
      allowedExtensions: const ['jar'],
    );
    if (files.isEmpty) {
      return null;
    }
    return files.first.path;
  }

  Future<void> _installJava(int major) async {
    setState(() {
      _busy = true;
      _progress.clear();
    });
    try {
      await _collect(installJava(major: major));
      final runtimes = await listRuntimes();
      if (mounted) {
        setState(() => _runtimes = runtimes);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _install(Stream<ProgressEvent> stream) async {
    setState(() {
      _busy = true;
      _progress.clear();
    });
    try {
      final id = await _collect(stream);
      if (mounted && id != null) {
        Navigator.pop(context, true);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _progress.add(readableError(error)));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<String?> _collect(Stream<ProgressEvent> stream) async {
    String? serverId;
    await for (final event in stream) {
      if (!mounted) {
        break;
      }
      setState(() => _progress.add('${event.stage}: ${event.message}'));
      if (event.error != null) {
        throw Exception(event.error);
      }
      serverId = event.serverId ?? serverId;
    }
    return serverId;
  }
}

class _ModeCard extends StatelessWidget {
  const _ModeCard({required this.selected, required this.icon, required this.title, required this.subtitle, required this.onTap});

  final bool selected;
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Material(
        color: panelCard,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: selected ? panelAccent : panelCardBorder, width: selected ? 1.5 : 1),
            ),
            child: Row(
              children: [
                Icon(icon, color: selected ? panelAccent : panelMuted),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(title, style: const TextStyle(fontWeight: FontWeight.w700)),
                      Text(subtitle, style: const TextStyle(color: panelMuted, fontSize: 12)),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
