// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/screens/create/create_wizard.dart';
import 'package:voxel_panel/screens/create/modpack_form.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/common/section_header.dart';
import 'package:voxel_panel/widgets/provider_icon.dart';

class CreateServerScreen extends StatefulWidget {
  const CreateServerScreen({super.key, this.startOnImport = false});

  final bool startOnImport;

  @override
  State<CreateServerScreen> createState() => _CreateServerScreenState();
}

enum _CreateMode { create, modpack, import }

class _CreateServerScreenState extends State<CreateServerScreen> {
  late var _mode = widget.startOnImport ? _CreateMode.import : _CreateMode.create;
  var _busy = false;
  final _importPath = TextEditingController();
  ImportPreview? _preview;
  var _acceptEula = false;

  @override
  void dispose() {
    _importPath.dispose();
    super.dispose();
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
                      selected: _mode == _CreateMode.create,
                      icon: Icons.view_in_ar,
                      title: l.createMode,
                      subtitle: l.createModeSubtitle,
                      onTap: _busy ? null : () => setState(() => _mode = _CreateMode.create),
                    ),
                    const SizedBox(width: 12),
                    _ModeCard(
                      selected: _mode == _CreateMode.modpack,
                      icon: Icons.inventory_2_outlined,
                      title: l.modpackMode,
                      subtitle: l.modpackModeSubtitle,
                      onTap: _busy ? null : () => setState(() => _mode = _CreateMode.modpack),
                    ),
                    const SizedBox(width: 12),
                    _ModeCard(
                      selected: _mode == _CreateMode.import,
                      icon: Icons.download_outlined,
                      title: l.importServer,
                      subtitle: l.importModeSubtitle,
                      onTap: _busy ? null : () => setState(() => _mode = _CreateMode.import),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                switch (_mode) {
                  _CreateMode.create => CreateWizard(onCreated: (_) => Navigator.pop(context, true), onBusyChanged: (busy) => setState(() => _busy = busy)),
                  _CreateMode.modpack => ModpackForm(onCreated: (_) => Navigator.pop(context, true), onBusyChanged: (busy) => setState(() => _busy = busy)),
                  _CreateMode.import => _importForm(context),
                },
              ],
            ),
          ),
        ],
      ),
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
              Expanded(child: TextField(controller: _importPath, readOnly: true, decoration: const InputDecoration(prefixIcon: Icon(Icons.folder_outlined)))),
              const SizedBox(width: 8),
              OutlinedButton(onPressed: _busy ? null : _browseImport, child: Text(l.browse)),
            ],
          ),
          if (preview != null) ...[
            const SizedBox(height: 16),
            Row(
              children: [
                ProviderIcon(preview.provider, size: 44),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${providerName(preview.provider)} ${preview.mcVersion.isEmpty ? '' : preview.mcVersion}',
                        style: const TextStyle(fontWeight: FontWeight.w700),
                      ),
                      Text(l.importDetectedJava(preview.javaMajor == 0 ? l.notDetected : '${preview.javaMajor}'), style: TextStyle(color: context.voxel.muted)),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(l.importDetectedRam(preview.ramMin, preview.ramMax)),
            Text(l.importDetectedContent(preview.pluginCount, preview.modCount, preview.worldCount)),
            if (preview.jarPath.isEmpty) Text(l.importNoJar, style: TextStyle(color: context.voxel.warning)),
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
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16), side: BorderSide(color: selected ? colors.accent : colors.cardBorder)),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(16),
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
