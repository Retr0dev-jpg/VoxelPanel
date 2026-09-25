// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/common/section_header.dart';
import 'package:voxel_panel/widgets/create_wizard_body.dart';

class CreateServerScreen extends ConsumerStatefulWidget {
  const CreateServerScreen({super.key, this.startOnImport = false});

  final bool startOnImport;

  @override
  ConsumerState<CreateServerScreen> createState() => _CreateServerScreenState();
}

class _CreateServerScreenState extends ConsumerState<CreateServerScreen> {
  List<String> _versions = [];
  List<RamChoice> _ram = [];
  List<JvmPresetInfo> _presets = [];
  var _defaultPreset = JvmPreset.aikar;
  List<JavaRuntimeInfo> _runtimes = [];
  List<JavaReleaseInfo> _releases = [];
  var _min = '2G';
  var _max = '4G';
  var _ready = false;
  var _busy = false;
  var _importing = false;
  Object? _error;
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
    setState(() {
      _error = null;
      _ready = false;
    });
    try {
      final results = await Future.wait([listPaperVersions(), listJavaReleases(), listRuntimes()]);
      final suggestion = suggestRam();
      final defaults = (await ref.read(launcherSettingsProvider.future)).defaults;
      if (!mounted) {
        return;
      }
      setState(() {
        _versions = results[0] as List<String>;
        _releases = results[1] as List<JavaReleaseInfo>;
        _runtimes = results[2] as List<JavaRuntimeInfo>;
        _ram = ramPresets();
        _presets = jvmPresets();
        _defaultPreset = defaults.jvmPreset;
        _min = defaults.ramMin.isEmpty ? suggestion.ramMin : defaults.ramMin;
        _max = defaults.ramMax.isEmpty ? suggestion.ramMax : defaults.ramMax;
        _ready = true;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(entries: [SidebarEntry(label: l.navServers, icon: Icons.dns_outlined)], selected: 0, onSelected: (_) {}),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.fromLTRB(28, 18, 28, 28),
              children: [
                SectionHeader(
                  title: l.newServer,
                  subtitle: l.createSubtitle,
                  leading: IconButton(onPressed: _busy ? null : () => Navigator.pop(context), icon: const Icon(Icons.arrow_back)),
                ),
                const SizedBox(height: 20),
                Row(
                  children: [
                    _ModeCard(
                      selected: !_importing,
                      icon: Icons.view_in_ar,
                      title: l.createMode,
                      subtitle: l.createModeSubtitle,
                      onTap: _busy ? null : () => setState(() => _importing = false),
                    ),
                    const SizedBox(width: 12),
                    _ModeCard(
                      selected: _importing,
                      icon: Icons.download_outlined,
                      title: l.importServer,
                      subtitle: l.importModeSubtitle,
                      onTap: _busy ? null : () => setState(() => _importing = true),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                if (_importing) _importForm(context) else _createForm(context),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _createForm(BuildContext context) {
    if (_error != null) {
      return PanelCard(child: ErrorState(error: _error!, onRetry: _load));
    }
    if (!_ready) {
      return const Padding(padding: EdgeInsets.all(40), child: Center(child: CircularProgressIndicator()));
    }
    return CreateWizardBody(
      paperVersions: _versions,
      ramChoices: _ram,
      jvmPresets: _presets,
      defaultPreset: _defaultPreset,
      runtimes: _runtimes,
      javaReleases: _releases,
      suggestedMin: _min,
      suggestedMax: _max,
      progress: _progress,
      busy: _busy,
      onAuto: (request) => _install(installAuto(request: request)),
      onManual: (request) => _install(installManual(request: request)),
      pickDirectory: () => FilePicker.getDirectoryPath(dialogTitle: context.l10n.serverFolder),
      pickJar: _pickJar,
      installJava: _installJava,
    );
  }

  Widget _importForm(BuildContext context) {
    final l = context.l10n;
    final preview = _preview;
    return PanelCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.serverFolder, style: const TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: TextField(controller: _importPath, readOnly: true, decoration: const InputDecoration(prefixIcon: Icon(Icons.folder_outlined))),
              ),
              const SizedBox(width: 8),
              OutlinedButton(onPressed: _busy ? null : _browseImport, child: Text(l.browse)),
            ],
          ),
          if (preview != null) ...[
            const SizedBox(height: 12),
            Text(l.importDetectedPaper(preview.paperVersion.isEmpty ? l.notDetected : preview.paperVersion)),
            Text(l.importDetectedJava(preview.javaMajor == 0 ? l.notDetected : '${preview.javaMajor}')),
            Text(l.importDetectedRam(preview.ramMin, preview.ramMax)),
            Text(l.importDetectedContent(preview.pluginCount, preview.worldCount)),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              value: _acceptEula,
              onChanged: (value) => setState(() => _acceptEula = value ?? false),
              title: Text(l.eulaCheckbox),
            ),
          ],
          const SizedBox(height: 12),
          FilledButton(onPressed: _busy || preview == null ? null : _confirmImport, child: Text(l.importServer)),
        ],
      ),
    );
  }

  Future<void> _browseImport() async {
    final path = await FilePicker.getDirectoryPath(dialogTitle: context.l10n.serverFolder);
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
        showError(context, error);
      }
    }
  }

  Future<void> _confirmImport() async {
    final preview = _preview;
    if (preview == null) {
      return;
    }
    setState(() => _busy = true);
    final ok = await runGuarded(context, () => importServer(path: preview.root, name: preview.name, acceptEula: _acceptEula));
    if (!mounted) {
      return;
    }
    setState(() => _busy = false);
    if (ok) {
      Navigator.pop(context, true);
    }
  }

  Future<String?> _pickJar() async {
    final files = await FilePicker.pickFiles(dialogTitle: context.l10n.serverJarTitle, type: FileType.custom, allowedExtensions: const ['jar']);
    return files.isEmpty ? null : files.first.path;
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
        showError(context, error);
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
        setState(() => _progress.add(describeError(context, error)));
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
      setState(() {
        final line = '${event.stage}: ${event.message}';
        // Download progress updates the previous line instead of flooding the log.
        if (_progress.isNotEmpty && event.fraction != null && !event.done && _progress.last.startsWith('${event.stage}: ')) {
          _progress[_progress.length - 1] = line;
        } else {
          _progress.add(line);
        }
      });
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
    final colors = context.voxel;
    return Expanded(
      child: Material(
        color: selected ? colors.accent.withValues(alpha: 0.18) : colors.card,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: selected ? colors.accent : colors.cardBorder),
            ),
            child: Row(
              children: [
                Icon(icon, color: selected ? colors.accent : colors.muted),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(title, style: const TextStyle(fontWeight: FontWeight.w700)),
                      Text(subtitle, style: TextStyle(color: colors.muted, fontSize: 12)),
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
