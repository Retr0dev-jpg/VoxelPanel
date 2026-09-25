// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

class BackupsTab extends StatefulWidget {
  const BackupsTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<BackupsTab> createState() => _BackupsTabState();
}

class _BackupsTabState extends State<BackupsTab> {
  List<BackupInfo>? _backups;
  Object? _error;
  var _busy = false;
  var _message = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final backups = await listBackups(id: widget.serverId);
      if (mounted) {
        setState(() {
          _backups = backups;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 12, 28, 8),
          child: Row(
            children: [
              FilledButton.icon(
                onPressed: _busy ? null : () => _run(createBackup(id: widget.serverId)),
                icon: const Icon(Icons.archive_outlined),
                label: Text(l.createBackup),
              ),
              const SizedBox(width: 8),
              IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  [if (widget.running) l.backupRunningHint, if (_message.isNotEmpty) _message].join(' '),
                  style: TextStyle(color: colors.muted),
                ),
              ),
            ],
          ),
        ),
        if (_busy) const LinearProgressIndicator(),
        Expanded(child: _body(context)),
      ],
    );
  }

  Widget _body(BuildContext context) {
    final l = context.l10n;
    final backups = _backups;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (backups == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (backups.isEmpty) {
      return EmptyState(icon: Icons.inventory_2_outlined, message: l.noBackups);
    }
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      itemCount: backups.length,
      itemBuilder: (context, index) {
        final backup = backups[index];
        final created = DateTime.fromMillisecondsSinceEpoch(backup.createdMs).toLocal().toString().substring(0, 19);
        return ListTile(
          leading: const Icon(Icons.inventory_2_outlined),
          title: Text(backup.fileName),
          subtitle: Text('$created · ${formatBytes(backup.sizeBytes)}'),
          trailing: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              TextButton(
                onPressed: widget.running || _busy
                    ? null
                    : () async {
                        final ok = await confirmAction(context, title: l.restoreBackupTitle, message: l.restoreBackupMessage, confirmLabel: l.restore);
                        if (ok) {
                          await _run(restoreBackup(id: widget.serverId, fileName: backup.fileName));
                        }
                      },
                child: Text(l.restore),
              ),
              IconButton(
                tooltip: l.delete,
                onPressed: _busy
                    ? null
                    : () async {
                        final ok = await confirmAction(context, title: l.deleteBackupTitle, message: backup.fileName, destructive: true, confirmLabel: l.delete);
                        if (ok && context.mounted) {
                          await runGuarded(context, () => deleteBackup(id: widget.serverId, fileName: backup.fileName));
                          await _load();
                        }
                      },
                icon: const Icon(Icons.delete_outline),
              ),
            ],
          ),
        );
      },
    );
  }

  Future<void> _run(Stream<ProgressEvent> stream) async {
    setState(() {
      _busy = true;
      _message = '';
    });
    try {
      await for (final event in stream) {
        if (!mounted) {
          return;
        }
        setState(() => _message = event.message);
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      await _load();
    } catch (error) {
      if (mounted) {
        setState(() => _message = describeError(context, error));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }
}
