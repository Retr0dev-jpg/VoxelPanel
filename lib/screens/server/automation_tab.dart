// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/automation.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

String scheduleKindLabel(AppLocalizations l, ScheduleKind kind) => switch (kind) {
  ScheduleKind.restart => l.scheduleRestart,
  ScheduleKind.command => l.scheduleCommand,
  ScheduleKind.backup => l.scheduleBackup,
  ScheduleKind.start => l.scheduleStart,
  ScheduleKind.stop => l.scheduleStop,
};

IconData scheduleKindIcon(ScheduleKind kind) => switch (kind) {
  ScheduleKind.restart => Icons.restart_alt,
  ScheduleKind.command => Icons.terminal,
  ScheduleKind.backup => Icons.archive_outlined,
  ScheduleKind.start => Icons.play_arrow,
  ScheduleKind.stop => Icons.stop,
};

/// Common schedules offered as one-click presets.
const cronPresets = <String>['0 4 * * *', '0 */6 * * *', '0 * * * *', '*/30 * * * *', '0 5 * * 1'];

String cronPresetLabel(AppLocalizations l, String cron) => switch (cron) {
  '0 4 * * *' => l.cronDaily4,
  '0 */6 * * *' => l.cronEvery6h,
  '0 * * * *' => l.cronHourly,
  '*/30 * * * *' => l.cronEvery30m,
  '0 5 * * 1' => l.cronWeekly,
  _ => cron,
};

String formatRun(int unix) {
  final time = DateTime.fromMillisecondsSinceEpoch(unix * 1000).toLocal();
  String two(int value) => value.toString().padLeft(2, '0');
  return '${two(time.day)}/${two(time.month)} ${two(time.hour)}:${two(time.minute)}';
}

class AutomationTab extends StatefulWidget {
  const AutomationTab({super.key, required this.serverId, required this.autoRestart, required this.autostart});

  final String serverId;
  final bool autoRestart;
  final bool autostart;

  @override
  State<AutomationTab> createState() => _AutomationTabState();
}

class _AutomationTabState extends State<AutomationTab> {
  List<ScheduledTask>? _tasks;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final tasks = await listSchedules(id: widget.serverId);
      if (mounted) {
        setState(() {
          _tasks = tasks;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _save(List<ScheduledTask> tasks) async {
    if (await runGuarded(context, () => saveSchedules(id: widget.serverId, tasks: tasks), success: context.l10n.schedulesSaved)) {
      await _load();
    }
  }

  Future<void> _edit([ScheduledTask? task]) async {
    final result = await showDialog<ScheduledTask>(context: context, builder: (context) => _TaskDialog(task: task));
    if (result == null) {
      return;
    }
    final tasks = [...?_tasks];
    final index = tasks.indexWhere((item) => item.id == result.id && result.id.isNotEmpty);
    index >= 0 ? tasks[index] = result : tasks.add(result);
    await _save(tasks);
  }

  ScheduledTask _copy(ScheduledTask task, {bool? enabled}) => ScheduledTask(
    id: task.id,
    kind: task.kind,
    cron: task.cron,
    command: task.command,
    warnPlayers: task.warnPlayers,
    enabled: enabled ?? task.enabled,
    lastRunUnix: task.lastRunUnix,
  );

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final tasks = _tasks;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (tasks == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 12, 28, 28),
      children: [
        PanelCard(
          title: l.automationStatus,
          icon: Icons.bolt,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('${widget.autoRestart ? '✓' : '✗'} ${l.serverAutoRestart}'),
              Text('${widget.autostart ? '✓' : '✗'} ${l.serverAutostart}'),
              const SizedBox(height: 6),
              Text(l.automationStatusHint, style: TextStyle(color: colors.muted, fontSize: 12)),
            ],
          ),
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            Expanded(child: Text(l.scheduledTasks, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700))),
            FilledButton.icon(onPressed: () => _edit(), icon: const Icon(Icons.add), label: Text(l.addTask)),
          ],
        ),
        const SizedBox(height: 8),
        if (tasks.isEmpty) EmptyState(icon: Icons.event_repeat, message: l.noTasks),
        for (final task in tasks) ...[
          PanelCard(
            child: Row(
              children: [
                Icon(scheduleKindIcon(task.kind), color: task.enabled ? colors.accent : colors.muted),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${scheduleKindLabel(l, task.kind)}${task.kind == ScheduleKind.command ? ': ${task.command}' : ''}',
                        style: const TextStyle(fontWeight: FontWeight.w700),
                      ),
                      Text('${cronPresetLabel(l, task.cron)} · ${task.cron}', style: TextStyle(color: colors.muted, fontSize: 12)),
                      Builder(
                        builder: (context) {
                          try {
                            final next = [for (final value in nextScheduleRuns(cron: task.cron, count: 1)) value.toInt()];
                            return Text(
                              [
                                if (next.isNotEmpty) l.nextRun(formatRun(next.first)),
                                if (task.lastRunUnix != null) l.lastRun(formatRun(task.lastRunUnix!)),
                              ].join(' · '),
                              style: TextStyle(color: colors.muted, fontSize: 12),
                            );
                          } catch (error) {
                            return Text(describeError(context, error), style: TextStyle(color: colors.danger, fontSize: 12));
                          }
                        },
                      ),
                    ],
                  ),
                ),
                Switch(value: task.enabled, onChanged: (value) => _save([for (final item in tasks) item.id == task.id ? _copy(item, enabled: value) : item])),
                PopupMenuButton<String>(
                  onSelected: (action) async {
                    switch (action) {
                      case 'edit':
                        await _edit(task);
                      case 'run':
                        await runGuarded(context, () => runScheduleNow(id: widget.serverId, taskId: task.id), success: l.taskStarted);
                        await _load();
                      case 'delete':
                        await _save([for (final item in tasks) if (item.id != task.id) item]);
                    }
                  },
                  itemBuilder: (context) => [
                    PopupMenuItem(value: 'edit', child: Text(l.edit)),
                    PopupMenuItem(value: 'run', child: Text(l.runNow)),
                    PopupMenuItem(value: 'delete', child: Text(l.delete, style: TextStyle(color: colors.danger))),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
        ],
      ],
    );
  }
}

class _TaskDialog extends StatefulWidget {
  const _TaskDialog({this.task});

  final ScheduledTask? task;

  @override
  State<_TaskDialog> createState() => _TaskDialogState();
}

class _TaskDialogState extends State<_TaskDialog> {
  late var _kind = widget.task?.kind ?? ScheduleKind.restart;
  late final _cron = TextEditingController(text: widget.task?.cron ?? cronPresets.first);
  late final _command = TextEditingController(text: widget.task?.command ?? '');
  late var _warn = widget.task?.warnPlayers ?? true;

  @override
  void dispose() {
    _cron.dispose();
    _command.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    List<int> preview = const [];
    String? issue;
    try {
      preview = [for (final value in nextScheduleRuns(cron: _cron.text, count: 3)) value.toInt()];
    } catch (error) {
      issue = describeError(context, error);
    }
    return AlertDialog(
      title: Text(widget.task == null ? l.addTask : l.editTask),
      content: SizedBox(
        width: 480,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            DropdownButtonFormField<ScheduleKind>(
              initialValue: _kind,
              decoration: InputDecoration(labelText: l.taskKind),
              items: [for (final kind in ScheduleKind.values) DropdownMenuItem(value: kind, child: Row(children: [Icon(scheduleKindIcon(kind), size: 18), const SizedBox(width: 8), Text(scheduleKindLabel(l, kind))]))],
              onChanged: (value) => setState(() => _kind = value ?? _kind),
            ),
            if (_kind == ScheduleKind.command) ...[
              const SizedBox(height: 12),
              TextField(controller: _command, decoration: InputDecoration(labelText: l.scheduleCommand, hintText: 'say Backup tra poco!')),
            ],
            if (_kind == ScheduleKind.restart)
              SwitchListTile(contentPadding: EdgeInsets.zero, title: Text(l.warnPlayers), subtitle: Text(l.warnPlayersHint), value: _warn, onChanged: (value) => setState(() => _warn = value)),
            const SizedBox(height: 12),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [for (final preset in cronPresets) ActionChip(label: Text(cronPresetLabel(l, preset)), onPressed: () => setState(() => _cron.text = preset))],
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _cron,
              onChanged: (_) => setState(() {}),
              style: const TextStyle(fontFamily: 'monospace'),
              decoration: InputDecoration(labelText: l.cronExpression, helperText: l.cronHelp, errorText: issue),
            ),
            if (preview.isNotEmpty) ...[
              const SizedBox(height: 8),
              Text(l.nextRuns(preview.map(formatRun).join(', ')), style: TextStyle(color: colors.muted, fontSize: 12)),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel)),
        FilledButton(
          onPressed: issue != null || (_kind == ScheduleKind.command && _command.text.trim().isEmpty)
              ? null
              : () => Navigator.pop(
                  context,
                  ScheduledTask(
                    id: widget.task?.id ?? '',
                    kind: _kind,
                    cron: _cron.text.trim(),
                    command: _command.text.trim(),
                    warnPlayers: _warn,
                    enabled: widget.task?.enabled ?? true,
                    lastRunUnix: widget.task?.lastRunUnix,
                  ),
                ),
          child: Text(l.save),
        ),
      ],
    );
  }
}
