// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

/// Returns true when the server software was replaced.
Future<bool> showChangeVersionDialog(BuildContext context, ServerDetails details) async {
  final changed = await showDialog<bool>(context: context, barrierDismissible: false, builder: (context) => _ChangeVersionDialog(details: details));
  return changed ?? false;
}

class _ChangeVersionDialog extends StatefulWidget {
  const _ChangeVersionDialog({required this.details});

  final ServerDetails details;

  @override
  State<_ChangeVersionDialog> createState() => _ChangeVersionDialogState();
}

class _ChangeVersionDialogState extends State<_ChangeVersionDialog> {
  late final ProviderInfo _info = providerInfo(kind: widget.details.provider);
  var _snapshots = false;
  late Future<List<VersionEntry>> _versions = listVersions(provider: widget.details.provider, includeSnapshots: false);
  late var _version = widget.details.mcVersion ?? '';
  Future<List<BuildEntry>>? _builds;
  var _build = '';
  var _busy = false;
  final _log = <String>[];

  @override
  void initState() {
    super.initState();
    if (_info.hasBuilds && _version.isNotEmpty) {
      _builds = listBuilds(provider: widget.details.provider, version: _version);
    }
  }

  Future<void> _apply() async {
    setState(() {
      _busy = true;
      _log.clear();
    });
    try {
      await for (final event in changeServerVersion(id: widget.details.id, mcVersion: _version, build: _build)) {
        if (!mounted) {
          return;
        }
        setState(() => _log.add(event.message));
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      if (mounted) {
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
    return AlertDialog(
      title: Text(l.changeVersionTitle(_info.name)),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(l.changeVersionHint, style: TextStyle(color: context.voxel.muted)),
            const SizedBox(height: 12),
            if (_info.hasSnapshots)
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(l.showSnapshots),
                value: _snapshots,
                onChanged: _busy
                    ? null
                    : (value) => setState(() {
                        _snapshots = value;
                        _versions = listVersions(provider: widget.details.provider, includeSnapshots: value);
                      }),
              ),
            FutureBuilder<List<VersionEntry>>(
              future: _versions,
              builder: (context, snapshot) {
                if (snapshot.hasError) {
                  return Text(describeError(context, snapshot.error!));
                }
                final versions = snapshot.data;
                if (versions == null) {
                  return const LinearProgressIndicator();
                }
                return DropdownButtonFormField<String>(
                  initialValue: versions.any((entry) => entry.id == _version) ? _version : null,
                  isExpanded: true,
                  decoration: InputDecoration(labelText: l.stepVersion),
                  items: [for (final entry in versions) DropdownMenuItem(value: entry.id, child: Text(entry.id))],
                  onChanged: _busy
                      ? null
                      : (value) => setState(() {
                          _version = value ?? _version;
                          _build = '';
                          _builds = _info.hasBuilds ? listBuilds(provider: widget.details.provider, version: _version) : null;
                        }),
                );
              },
            ),
            if (_builds != null) ...[
              const SizedBox(height: 12),
              FutureBuilder<List<BuildEntry>>(
                future: _builds,
                builder: (context, snapshot) => DropdownButtonFormField<String>(
                  key: ValueKey('change-builds-$_version-${snapshot.data?.length}'),
                  initialValue: _build,
                  isExpanded: true,
                  decoration: InputDecoration(labelText: l.buildLabel),
                  items: [
                    DropdownMenuItem(value: '', child: Text(l.latestBuild)),
                    for (final build in snapshot.data ?? const <BuildEntry>[]) DropdownMenuItem(value: build.id, child: Text(build.label)),
                  ],
                  onChanged: _busy ? null : (value) => setState(() => _build = value ?? ''),
                ),
              ),
            ],
            if (_log.isNotEmpty) ...[
              const SizedBox(height: 12),
              if (_busy) const LinearProgressIndicator(),
              const SizedBox(height: 8),
              for (final line in _log.reversed.take(4).toList().reversed) Text(line, style: const TextStyle(fontSize: 12)),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: _busy ? null : () => Navigator.pop(context, false), child: Text(l.cancel)),
        AsyncActionButton(label: l.changeVersionConfirm, onPressed: _busy || _version.isEmpty ? null : _apply),
      ],
    );
  }
}
