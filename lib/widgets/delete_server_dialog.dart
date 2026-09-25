// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/theme.dart';

class DeleteServerOptions {
  const DeleteServerOptions({required this.deleteFiles, required this.deleteBackups});

  final bool deleteFiles;
  final bool deleteBackups;
}

/// Asks what to remove. Returns null when the user cancels.
Future<DeleteServerOptions?> showDeleteServerDialog(BuildContext context, {required List<String> names}) {
  return showDialog<DeleteServerOptions>(context: context, builder: (context) => _DeleteServerDialog(names: names));
}

class _DeleteServerDialog extends StatefulWidget {
  const _DeleteServerDialog({required this.names});

  final List<String> names;

  @override
  State<_DeleteServerDialog> createState() => _DeleteServerDialogState();
}

class _DeleteServerDialogState extends State<_DeleteServerDialog> {
  var _files = false;
  var _backups = false;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return AlertDialog(
      title: Text(widget.names.length == 1 ? l.deleteServerTitle(widget.names.first) : l.deleteServersTitle(widget.names.length)),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l.deleteServerMessage),
            const SizedBox(height: 8),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              value: _files,
              onChanged: (value) => setState(() => _files = value ?? false),
              title: Text(l.deleteServerFiles),
              subtitle: Text(l.deleteServerFilesHint, style: TextStyle(color: colors.danger)),
            ),
            CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              value: _backups,
              onChanged: (value) => setState(() => _backups = value ?? false),
              title: Text(l.deleteServerBackups),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel)),
        FilledButton(
          style: actionButtonStyle(colors.stop),
          onPressed: () => Navigator.pop(context, DeleteServerOptions(deleteFiles: _files, deleteBackups: _backups)),
          child: Text(l.delete),
        ),
      ],
    );
  }
}
