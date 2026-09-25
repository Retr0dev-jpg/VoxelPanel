// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/content.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

String sourceLabel(ContentSourceKind source) => switch (source) {
  ContentSourceKind.modrinth => 'Modrinth',
  ContentSourceKind.hangar => 'Hangar',
  ContentSourceKind.spiget => 'SpigotMC',
  ContentSourceKind.curseForge => 'CurseForge',
};

String compactCount(int value) {
  if (value >= 1000000) {
    return '${(value / 1000000).toStringAsFixed(1)}M';
  }
  if (value >= 1000) {
    return '${(value / 1000).toStringAsFixed(1)}k';
  }
  return '$value';
}

/// Icon of a catalogue project; falls back to a generic icon when missing or broken.
class ProjectIcon extends StatelessWidget {
  const ProjectIcon(this.url, {super.key, this.size = 44});

  final String url;
  final double size;

  @override
  Widget build(BuildContext context) {
    final fallback = Container(
      width: size,
      height: size,
      decoration: BoxDecoration(color: context.voxel.field, borderRadius: BorderRadius.circular(10)),
      child: Icon(Icons.extension, color: context.voxel.muted),
    );
    if (url.isEmpty) {
      return fallback;
    }
    return ClipRRect(
      borderRadius: BorderRadius.circular(10),
      child: Image.network(url, width: size, height: size, fit: BoxFit.cover, errorBuilder: (context, error, stack) => fallback),
    );
  }
}

/// Catalogue browser for plugins or mods of one server. Pops `true` when something was installed.
class ContentBrowserPage extends StatefulWidget {
  const ContentBrowserPage({super.key, required this.serverId, required this.kind});

  final String serverId;
  final AddonKind kind;

  @override
  State<ContentBrowserPage> createState() => _ContentBrowserPageState();
}

class _ContentBrowserPageState extends State<ContentBrowserPage> {
  List<ContentSourceKind> _sources = [];
  ContentSourceKind? _source;
  final _query = TextEditingController();
  final _projects = <ContentProject>[];
  var _total = 0;
  var _page = 0;
  var _loading = false;
  Object? _error;
  var _installed = false;

  @override
  void initState() {
    super.initState();
    _init();
  }

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  Future<void> _init() async {
    try {
      final sources = await contentSources(id: widget.serverId, kind: widget.kind);
      if (!mounted) {
        return;
      }
      setState(() {
        _sources = sources;
        _source = sources.firstOrNull;
      });
      await _search(reset: true);
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _search({required bool reset}) async {
    final source = _source;
    if (source == null) {
      return;
    }
    setState(() {
      _loading = true;
      _error = null;
      if (reset) {
        _projects.clear();
        _page = 0;
      }
    });
    try {
      final result = await searchContent(id: widget.serverId, kind: widget.kind, source: source, query: _query.text, page: _page);
      if (!mounted) {
        return;
      }
      setState(() {
        _projects.addAll(result.projects);
        _total = result.total;
        _page++;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _open(ContentProject project) async {
    final installed = await showDialog<bool>(context: context, builder: (context) => _VersionDialog(serverId: widget.serverId, kind: widget.kind, project: project));
    if (installed == true) {
      _installed = true;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) {
          Navigator.pop(context, _installed);
        }
      },
      child: Scaffold(
        body: Padding(
          padding: const EdgeInsets.fromLTRB(28, 20, 28, 20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  IconButton(onPressed: () => Navigator.pop(context, _installed), icon: const Icon(Icons.arrow_back)),
                  const SizedBox(width: 8),
                  Text(widget.kind == AddonKind.mod ? l.browseMods : l.browsePlugins, style: const TextStyle(fontSize: 26, fontWeight: FontWeight.w700)),
                ],
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  for (final source in _sources)
                    ChoiceChip(
                      label: Text(sourceLabel(source)),
                      selected: _source == source,
                      onSelected: (_) {
                        setState(() => _source = source);
                        _search(reset: true);
                      },
                    ),
                  if (!_sources.contains(ContentSourceKind.curseForge)) Text(l.curseforgeKeyHint, style: TextStyle(color: colors.muted, fontSize: 12)),
                ],
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _query,
                decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.searchCatalog, suffixIcon: IconButton(onPressed: () => _search(reset: true), icon: const Icon(Icons.arrow_forward))),
                onSubmitted: (_) => _search(reset: true),
              ),
              const SizedBox(height: 12),
              if (_loading && _projects.isEmpty) const LinearProgressIndicator(),
              Expanded(
                child: _error != null && _projects.isEmpty
                    ? ErrorState(error: _error!, onRetry: () => _search(reset: true))
                    : !_loading && _projects.isEmpty
                    ? EmptyState(icon: Icons.search_off, message: l.noSearchResultsCatalog)
                    : ListView.separated(
                        itemCount: _projects.length + 1,
                        separatorBuilder: (context, index) => const SizedBox(height: 6),
                        itemBuilder: (context, index) {
                          if (index == _projects.length) {
                            return _projects.length < _total
                                ? Center(
                                    child: Padding(
                                      padding: const EdgeInsets.all(12),
                                      child: OutlinedButton(onPressed: _loading ? null : () => _search(reset: false), child: Text(_loading ? l.loading : l.loadMore)),
                                    ),
                                  )
                                : const SizedBox(height: 12);
                          }
                          final project = _projects[index];
                          return Material(
                            color: colors.card,
                            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12), side: BorderSide(color: colors.cardBorder)),
                            child: ListTile(
                              contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                              leading: ProjectIcon(project.iconUrl),
                              title: Text(project.title, style: const TextStyle(fontWeight: FontWeight.w700)),
                              subtitle: Text(
                                [if (project.author.isNotEmpty) project.author, project.description].join(' · '),
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                              ),
                              trailing: Wrap(
                                crossAxisAlignment: WrapCrossAlignment.center,
                                children: [
                                  Text('↓ ${compactCount(project.downloads)}', style: TextStyle(color: colors.muted)),
                                  if (project.pageUrl.isNotEmpty) IconButton(tooltip: l.openPage, onPressed: () => launchUrl(Uri.parse(project.pageUrl)), icon: const Icon(Icons.open_in_new, size: 18)),
                                ],
                              ),
                              onTap: () => _open(project),
                            ),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _VersionDialog extends StatefulWidget {
  const _VersionDialog({required this.serverId, required this.kind, required this.project});

  final String serverId;
  final AddonKind kind;
  final ContentProject project;

  @override
  State<_VersionDialog> createState() => _VersionDialogState();
}

class _VersionDialogState extends State<_VersionDialog> {
  late final Future<List<ContentVersion>> _versions = contentVersions(id: widget.serverId, kind: widget.kind, source: widget.project.source, projectId: widget.project.id);
  String? _selected;
  var _busy = false;
  final _log = <String>[];

  Future<void> _install() async {
    setState(() {
      _busy = true;
      _log.clear();
    });
    try {
      await for (final event in installContent(id: widget.serverId, kind: widget.kind, source: widget.project.source, projectId: widget.project.id, versionId: _selected ?? '')) {
        if (!mounted) {
          return;
        }
        setState(() => _log.add(event.message));
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      if (mounted) {
        showMessage(context, _log.lastOrNull ?? '');
        Navigator.pop(context, true);
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _log.add(describeError(context, error));
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return AlertDialog(
      title: Row(children: [ProjectIcon(widget.project.iconUrl, size: 32), const SizedBox(width: 12), Expanded(child: Text(widget.project.title))]),
      content: SizedBox(
        width: 560,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(widget.project.description, style: TextStyle(color: colors.muted)),
            const SizedBox(height: 12),
            FutureBuilder<List<ContentVersion>>(
              future: _versions,
              builder: (context, snapshot) {
                if (snapshot.hasError) {
                  return Text(describeError(context, snapshot.error!), style: TextStyle(color: colors.danger));
                }
                final versions = snapshot.data;
                if (versions == null) {
                  return const LinearProgressIndicator();
                }
                if (versions.isEmpty) {
                  return Text(l.noVersions);
                }
                final compatible = versions.where((version) => version.compatible).length;
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(l.compatibleVersions(compatible, versions.length)),
                    const SizedBox(height: 8),
                    DropdownButtonFormField<String>(
                      initialValue: _selected ?? '',
                      isExpanded: true,
                      decoration: InputDecoration(labelText: l.stepVersion),
                      items: [
                        DropdownMenuItem(value: '', child: Text(l.newestCompatible)),
                        for (final version in versions.take(40))
                          DropdownMenuItem(
                            value: version.id,
                            child: Text(
                              '${version.compatible ? '✓' : '✗'} ${version.name} · ${version.gameVersions.take(4).join(', ')}${version.stable ? '' : ' · beta'}',
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                      ],
                      onChanged: _busy ? null : (value) => setState(() => _selected = value == '' ? null : value),
                    ),
                  ],
                );
              },
            ),
            if (_log.isNotEmpty) ...[
              const SizedBox(height: 12),
              if (_busy) const LinearProgressIndicator(),
              for (final line in _log.reversed.take(5).toList().reversed) Text(line, style: const TextStyle(fontSize: 12)),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: _busy ? null : () => Navigator.pop(context, false), child: Text(l.close)),
        FilledButton.icon(onPressed: _busy ? null : _install, icon: const Icon(Icons.download), label: Text(l.install)),
      ],
    );
  }
}
