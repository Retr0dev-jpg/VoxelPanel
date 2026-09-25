// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:voxel_panel/screens/server/console_tab.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/server.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/minecraft_log.dart';

class LogsTab extends StatefulWidget {
  const LogsTab({super.key, required this.serverId});

  final String serverId;

  @override
  State<LogsTab> createState() => _LogsTabState();
}

class _LogsTabState extends State<LogsTab> {
  Future<List<LogFileInfo>>? _files;
  LogFileInfo? _selected;
  List<String> _lines = const [];
  var _loading = false;
  final _search = TextEditingController();
  final _levels = <LogLevel>{...LogLevel.values};

  @override
  void initState() {
    super.initState();
    _files = listLogs(id: widget.serverId);
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _open(LogFileInfo file) async {
    setState(() {
      _selected = file;
      _loading = true;
    });
    try {
      final text = await readLog(id: widget.serverId, relative: file.relative);
      if (mounted) {
        setState(() => _lines = text.split('\n'));
      }
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return LayoutBuilder(
      builder: (context, constraints) {
        final list = FutureBuilder<List<LogFileInfo>>(
          future: _files,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              return ErrorState(error: snapshot.error!);
            }
            final files = snapshot.data;
            if (files == null) {
              return const Center(child: CircularProgressIndicator());
            }
            if (files.isEmpty) {
              return EmptyState(icon: Icons.receipt_long_outlined, message: l.noLogs);
            }
            return ListView(
              children: [
                for (final file in files)
                  ListTile(
                    dense: true,
                    selected: _selected?.relative == file.relative,
                    leading: Icon(switch (file.kind) {
                      LogFileKind.latest => Icons.fiber_manual_record,
                      LogFileKind.archive => Icons.inventory_2_outlined,
                      LogFileKind.crash => Icons.bug_report_outlined,
                    }, color: file.kind == LogFileKind.crash ? colors.danger : null),
                    title: Text(file.name, overflow: TextOverflow.ellipsis),
                    subtitle: Text('${formatBytes(file.sizeBytes)} · ${DateTime.fromMillisecondsSinceEpoch(file.modifiedMs).toLocal().toString().substring(0, 16)}'),
                    onTap: () => _open(file),
                  ),
              ],
            );
          },
        );
        final viewer = _viewer(context);
        if (constraints.maxWidth < 900) {
          return _selected == null
              ? list
              : Column(
                  children: [
                    Align(alignment: Alignment.centerLeft, child: TextButton.icon(onPressed: () => setState(() => _selected = null), icon: const Icon(Icons.arrow_back), label: Text(l.back))),
                    Expanded(child: viewer),
                  ],
                );
        }
        return Row(
          children: [
            SizedBox(width: 300, child: list),
            const VerticalDivider(width: 1),
            Expanded(child: viewer),
          ],
        );
      },
    );
  }

  Widget _viewer(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final selected = _selected;
    if (selected == null) {
      return EmptyState(icon: Icons.article_outlined, message: l.selectLog);
    }
    final query = _search.text.trim().toLowerCase();
    final visible = _lines.where((line) => _levels.contains(levelOf(line)) && (query.isEmpty || line.toLowerCase().contains(query))).toList();
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SizedBox(
                width: 240,
                child: TextField(controller: _search, onChanged: (_) => setState(() {}), decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.consoleSearch, isDense: true)),
              ),
              for (final level in LogLevel.values)
                FilterChip(
                  label: Text(switch (level) {
                    LogLevel.info => l.logInfo,
                    LogLevel.warn => l.logWarn,
                    LogLevel.error => l.logError,
                  }),
                  selected: _levels.contains(level),
                  onSelected: (value) => setState(() => value ? _levels.add(level) : _levels.remove(level)),
                ),
              IconButton(tooltip: l.consoleCopy, onPressed: () => Clipboard.setData(ClipboardData(text: visible.join('\n'))), icon: const Icon(Icons.copy_all_outlined)),
              IconButton(
                tooltip: l.exportCopy,
                onPressed: () async {
                  final destination = await FilePicker.getDirectoryPath(dialogTitle: l.exportCopy);
                  if (destination != null && context.mounted) {
                    await runGuarded(context, () => exportServerPath(id: widget.serverId, relative: selected.relative, destinationDir: destination), success: l.exported);
                  }
                },
                icon: const Icon(Icons.download_outlined),
              ),
              IconButton(tooltip: l.refresh, onPressed: () => _open(selected), icon: const Icon(Icons.refresh)),
            ],
          ),
          const SizedBox(height: 8),
          if (_loading) const LinearProgressIndicator(),
          Expanded(
            child: Container(
              decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12)),
              child: SelectionArea(
                child: ListView.builder(
                  padding: const EdgeInsets.all(8),
                  itemCount: visible.length,
                  itemBuilder: (context, index) => DefaultTextStyle.merge(style: const TextStyle(color: Color(0xFFDDDDDD)), child: MinecraftLogLine(visible[index])),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
