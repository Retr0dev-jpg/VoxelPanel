// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/content/content_browser.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/content.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/content_layout.dart';

/// Plugins or mods of a server, depending on [kind].
class AddonsTab extends ConsumerStatefulWidget {
  const AddonsTab({super.key, required this.serverId, required this.running, required this.kind});

  final String serverId;
  final bool running;
  final AddonKind kind;

  @override
  ConsumerState<AddonsTab> createState() => _AddonsTabState();
}

class _AddonsTabState extends ConsumerState<AddonsTab> {
  List<AddonInfo>? _addons;
  Map<String, AddonUpdate> _updates = {};
  Object? _error;
  var _busy = false;
  var _filter = '';
  String _status = '';

  bool get _isMod => widget.kind == AddonKind.mod;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final addons = await listAddons(id: widget.serverId, kind: widget.kind);
      if (mounted) {
        setState(() {
          _addons = addons;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _act(Future<void> Function() action) async {
    setState(() => _busy = true);
    await runGuarded(context, action);
    await _load();
    if (mounted) {
      setState(() => _busy = false);
    }
  }

  Future<void> _checkUpdates() async {
    final l = context.l10n;
    setState(() {
      _busy = true;
      _status = l.checkingUpdates;
    });
    try {
      final updates = await checkAddonUpdates(id: widget.serverId, kind: widget.kind);
      if (mounted) {
        setState(() {
          _updates = {for (final update in updates) update.fileName: update};
          _status = updates.isEmpty ? l.allUpToDate : l.updatesAvailable(updates.length);
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _status = describeError(context, error));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _applyUpdates(List<AddonUpdate> updates) async {
    setState(() => _busy = true);
    try {
      await for (final event in updateAddons(id: widget.serverId, kind: widget.kind, updates: updates)) {
        if (!mounted) {
          return;
        }
        setState(() => _status = event.message);
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      setState(() => _updates = {});
    } catch (error) {
      if (mounted) {
        setState(() => _status = describeError(context, error));
      }
    } finally {
      await _load();
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _browse() async {
    final installed = await Navigator.push<bool>(context, MaterialPageRoute(builder: (context) => ContentBrowserPage(serverId: widget.serverId, kind: widget.kind)));
    if (installed == true) {
      await _load();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final locked = widget.running || _busy;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              FilledButton.icon(onPressed: locked ? null : _browse, icon: const Icon(Icons.travel_explore), label: Text(_isMod ? l.browseMods : l.browsePlugins)),
              OutlinedButton.icon(onPressed: locked ? null : _pickJar, icon: const Icon(Icons.upload_file), label: Text(_isMod ? l.installModJar : l.installJar)),
              OutlinedButton.icon(onPressed: _busy ? null : _checkUpdates, icon: const Icon(Icons.update), label: Text(l.checkUpdates)),
              if (_updates.isNotEmpty) FilledButton.tonalIcon(onPressed: locked ? null : () => _applyUpdates(_updates.values.toList()), icon: const Icon(Icons.system_update_alt), label: Text(l.updateAll(_updates.length))),
              SizedBox(
                width: 220,
                child: TextField(decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.filter, isDense: true), onChanged: (value) => setState(() => _filter = value.trim().toLowerCase())),
              ),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
              const ContentLayoutToggle(),
            ],
          ),
        ),
        if (widget.running) Padding(padding: const EdgeInsets.symmetric(horizontal: 28), child: Text(_isMod ? l.stopToEditMods : l.stopToEditPlugins, style: TextStyle(color: colors.warning))),
        if (_status.isNotEmpty) Padding(padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 4), child: Text(_status, style: TextStyle(color: colors.muted))),
        if (_busy) const LinearProgressIndicator(),
        Expanded(child: _body(context, locked)),
      ],
    );
  }

  Widget _body(BuildContext context, bool locked) {
    final l = context.l10n;
    final colors = context.voxel;
    final addons = _addons;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (addons == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (addons.isEmpty) {
      return EmptyState(icon: Icons.extension_outlined, message: _isMod ? l.noMods : l.noPlugins, action: FilledButton.icon(onPressed: locked ? null : _browse, icon: const Icon(Icons.travel_explore), label: Text(_isMod ? l.browseMods : l.browsePlugins)));
    }
    final visible = addons.where((addon) => _filter.isEmpty || addon.fileName.toLowerCase().contains(_filter) || addon.name.toLowerCase().contains(_filter)).toList();
    if (watchContentLayout(ref) == ContentLayout.grid) {
      return GridView.builder(
        padding: const EdgeInsets.fromLTRB(28, 8, 28, 20),
        gridDelegate: contentGridDelegate,
        itemCount: visible.length,
        itemBuilder: (context, index) => _AddonTile(
          addon: visible[index],
          update: _updates[visible[index].fileName],
          locked: locked,
          onToggle: (value) => _toggle(visible[index], value),
          onDelete: () => _delete(visible[index]),
          onUpdate: (update) => _applyUpdates([update]),
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      itemCount: visible.length,
      itemBuilder: (context, index) {
        final addon = visible[index];
        final update = _updates[addon.fileName];
        return ListTile(
          leading: Icon(Icons.extension, color: addon.enabled ? colors.accent : colors.muted),
          title: Row(
            children: [
              Flexible(child: Text(_title(addon), overflow: TextOverflow.ellipsis)),
              if (update != null) ...[
                const SizedBox(width: 8),
                ActionChip(
                  avatar: const Icon(Icons.arrow_upward, size: 14),
                  label: Text(update.newVersion),
                  onPressed: locked ? null : () => _applyUpdates([update]),
                ),
              ],
            ],
          ),
          subtitle: Text(
            [
              if (addon.name.isNotEmpty) addon.fileName,
              formatBytes(addon.sizeBytes),
              if (addon.authors.isNotEmpty) addon.authors.take(3).join(', '),
              if (addon.description.isNotEmpty) addon.description,
            ].join(' · '),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Switch(value: addon.enabled, onChanged: locked ? null : (value) => _toggle(addon, value)),
              IconButton(tooltip: l.delete, onPressed: locked ? null : () => _delete(addon), icon: const Icon(Icons.delete_outline)),
            ],
          ),
        );
      },
    );
  }

  Future<void> _toggle(AddonInfo addon, bool enabled) => _act(() => setAddonEnabled(id: widget.serverId, kind: widget.kind, fileName: addon.fileName, enabled: enabled));

  Future<void> _delete(AddonInfo addon) async {
    final l = context.l10n;
    final ok = await confirmAction(context, title: l.deletePluginTitle, message: addon.fileName, destructive: true, confirmLabel: l.delete);
    if (ok) {
      await _act(() => deleteAddon(id: widget.serverId, kind: widget.kind, fileName: addon.fileName));
    }
  }

  Future<void> _pickJar() async {
    final files = await FilePicker.pickFiles(dialogTitle: context.l10n.pluginJarTitle, type: FileType.custom, allowedExtensions: const ['jar']);
    final paths = files.map((file) => file.path).whereType<String>().toList();
    for (final path in paths) {
      await _act(() => installAddonFile(id: widget.serverId, kind: widget.kind, sourcePath: path));
    }
  }
}

String _title(AddonInfo addon) => addon.name.isEmpty ? addon.fileName : '${addon.name}${addon.version.isEmpty ? '' : ' ${addon.version}'}';

class _AddonTile extends StatelessWidget {
  const _AddonTile({required this.addon, required this.update, required this.locked, required this.onToggle, required this.onDelete, required this.onUpdate});

  final AddonInfo addon;
  final AddonUpdate? update;
  final bool locked;
  final ValueChanged<bool> onToggle;
  final VoidCallback onDelete;
  final ValueChanged<AddonUpdate> onUpdate;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final pending = update;
    return Material(
      color: colors.card,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14), side: BorderSide(color: colors.cardBorder)),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 12, 4, 4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.extension, size: 32, color: addon.enabled ? colors.accent : colors.muted),
                const Spacer(),
                if (pending != null)
                  ActionChip(avatar: const Icon(Icons.arrow_upward, size: 14), label: Text(pending.newVersion), onPressed: locked ? null : () => onUpdate(pending)),
                const SizedBox(width: 8),
              ],
            ),
            const SizedBox(height: 8),
            Text(_title(addon), maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontWeight: FontWeight.w700)),
            Text(
              [if (addon.name.isNotEmpty) addon.fileName, formatBytes(addon.sizeBytes)].join(' · '),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: colors.muted, fontSize: 11),
            ),
            const SizedBox(height: 4),
            Expanded(
              child: Text(
                [if (addon.authors.isNotEmpty) addon.authors.take(3).join(', '), if (addon.description.isNotEmpty) addon.description].join(' · '),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: colors.muted, fontSize: 12),
              ),
            ),
            Row(
              children: [
                Switch(value: addon.enabled, onChanged: locked ? null : onToggle),
                const Spacer(),
                IconButton(tooltip: l.delete, onPressed: locked ? null : onDelete, icon: const Icon(Icons.delete_outline)),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
