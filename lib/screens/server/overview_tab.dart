// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/provider_icon.dart';
import 'package:voxel_panel/widgets/sparkline.dart';

class OverviewTab extends ConsumerWidget {
  const OverviewTab({
    super.key,
    required this.details,
    required this.onChanged,
    required this.onDelete,
    required this.onRename,
    required this.onChangeVersion,
  });

  final ServerDetails details;
  final VoidCallback onChanged;
  final VoidCallback onDelete;
  final VoidCallback onRename;
  final VoidCallback onChangeVersion;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final colors = context.voxel;
    final runtime = ref.watch(serverRuntimeProvider(details.id));
    final history = ref.watch(statsHistoryProvider(details.id));
    final status = runtime?.status ?? ServerStatus.stopped;
    final ramMax = memoryBytes(details.ramMax);
    final memory = runtime?.memoryBytes ?? 0;
    final ramPercent = ramMax == null || ramMax == 0 ? 0.0 : (memory / ramMax * 100).clamp(0, 100).toDouble();
    final active = runtime?.pid != null;
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 20, 28, 28),
      children: [
        _Header(details: details, status: status, crashed: runtime?.crashed ?? false, onDelete: onDelete, onRename: onRename, onChangeVersion: onChangeVersion),
        if (runtime?.crashed == true && status == ServerStatus.stopped) ...[
          const SizedBox(height: 16),
          _Banner(icon: Icons.warning_amber, color: colors.danger, text: l.crashBanner(runtime?.lastExitCode ?? -1)),
        ],
        if (!details.eulaAccepted && !providerInfo(kind: details.provider).isProxy) ...[
          const SizedBox(height: 16),
          _Banner(
            icon: Icons.gavel,
            color: colors.warning,
            text: l.eulaMissing,
            action: FilledButton(
              onPressed: () async {
                if (await runGuarded(context, () => acceptServerEula(id: details.id))) {
                  onChanged();
                }
              },
              child: Text(l.acceptEula),
            ),
          ),
        ],
        const SizedBox(height: 20),
        ResponsiveGrid(
          minItemWidth: 190,
          children: [
            _StatCard(
              icon: Icons.people_outline,
              label: l.statPlayers,
              value: '${runtime?.players.length ?? 0} / ${details.maxPlayers}',
              hint: (runtime?.players.isEmpty ?? true) ? l.noPlayersOnline : runtime!.players.join(', '),
            ),
            _StatCard(
              icon: Icons.memory,
              label: l.statCpu,
              value: active ? '${runtime!.cpuPercent.toStringAsFixed(0)}%' : '--',
              hint: l.statCpuHint,
            ),
            _StatCard(
              icon: Icons.storage_outlined,
              label: l.statMemory,
              value: active ? formatBytes(memory) : '--',
              hint: l.statMemoryHint(details.ramMax),
            ),
            _StatCard(icon: Icons.schedule, label: l.statUptime, value: '', hint: l.statUptimeHint, live: _Uptime(startedUnix: runtime?.startedUnix)),
          ],
        ),
        const SizedBox(height: 16),
        PanelCard(
          title: l.liveStats,
          subtitle: l.liveStatsSubtitle,
          trailing: Text(status.isOnline ? l.serverOnline : l.serverOffline, style: TextStyle(color: status.isOnline ? colors.online : colors.muted)),
          child: ResponsiveGrid(
            minItemWidth: 280,
            children: [
              _Meter(
                title: l.statCpu,
                value: active ? '${runtime!.cpuPercent.toStringAsFixed(0)}%' : '--',
                percent: active ? runtime!.cpuPercent / 100 : 0,
                samples: [for (final sample in history) sample.cpuPercent],
              ),
              _Meter(
                title: l.statMemory,
                value: active ? '${formatBytes(memory)} / ${details.ramMax}' : '${details.ramMin} / ${details.ramMax}',
                percent: ramPercent / 100,
                samples: [
                  for (final sample in history) ramMax == null || ramMax == 0 ? 0 : (sample.memoryBytes / ramMax * 100).clamp(0, 100).toDouble(),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({
    required this.details,
    required this.status,
    required this.crashed,
    required this.onDelete,
    required this.onRename,
    required this.onChangeVersion,
  });

  final ServerDetails details;
  final ServerStatus status;
  final bool crashed;
  final VoidCallback onDelete;
  final VoidCallback onRename;
  final VoidCallback onChangeVersion;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final statusColor = switch (status) {
      ServerStatus.running => colors.online,
      ServerStatus.starting || ServerStatus.stopping => colors.warning,
      ServerStatus.stopped => crashed ? colors.danger : colors.muted,
    };
    final info = Row(
      children: [
        ProviderIcon(details.provider, size: 64),
        const SizedBox(width: 16),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(details.name, style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w700), overflow: TextOverflow.ellipsis),
              const SizedBox(height: 4),
              Row(
                children: [
                  Icon(Icons.circle, size: 10, color: statusColor),
                  const SizedBox(width: 8),
                  Text(statusLabel(context, status), style: TextStyle(color: statusColor)),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                l.serverInfoLine('${providerName(details.provider)} ${details.mcVersion ?? ''}${details.build == null ? '' : ' #${details.build}'}'.trim(), details.javaMajor?.toString() ?? l.notAvailable, details.ramMin, details.ramMax, details.port),
                style: TextStyle(color: colors.muted),
              ),
              SelectableText(details.root, style: TextStyle(color: colors.muted, fontSize: 12)),
            ],
          ),
        ),
      ],
    );
    final actions = Wrap(
      spacing: 8,
      runSpacing: 8,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        FilledButton.icon(
          style: actionButtonStyle(colors.start),
          onPressed: status.isActive ? null : () => runGuarded(context, () => startServer(id: details.id)),
          icon: const Icon(Icons.play_arrow),
          label: Text(l.actionStart),
        ),
        FilledButton(
          style: actionButtonStyle(colors.stop),
          onPressed: status == ServerStatus.running || status == ServerStatus.starting ? () => runGuarded(context, () => stopServer(id: details.id)) : null,
          child: Text(l.actionStop),
        ),
        FilledButton(
          style: actionButtonStyle(colors.restart),
          onPressed: status == ServerStatus.running ? () => runGuarded(context, () => restartServer(id: details.id)) : null,
          child: Text(l.actionRestart),
        ),
        OutlinedButton.icon(
          onPressed: () => runGuarded(context, () => openInExplorer(path: details.root)),
          icon: const Icon(Icons.folder_open_outlined),
          label: Text(l.openFolder),
        ),
        PopupMenuButton<String>(
          tooltip: l.moreActions,
          onSelected: (value) => switch (value) {
            'rename' => onRename(),
            'version' => onChangeVersion(),
            _ => onDelete(),
          },
          itemBuilder: (context) => [
            PopupMenuItem(value: 'rename', child: ListTile(leading: const Icon(Icons.edit_outlined), title: Text(l.renameServer))),
            PopupMenuItem(
              value: 'version',
              enabled: !status.isActive && details.provider != ProviderKind.custom,
              child: ListTile(leading: const Icon(Icons.system_update_alt), title: Text(l.changeVersion)),
            ),
            PopupMenuItem(
              value: 'delete',
              enabled: !status.isActive,
              child: ListTile(leading: Icon(Icons.delete_outline, color: colors.danger), title: Text(l.deleteServer)),
            ),
          ],
        ),
      ],
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth < 900) {
          return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [info, const SizedBox(height: 16), actions]);
        }
        return Row(children: [Expanded(child: info), const SizedBox(width: 12), Flexible(child: actions)]);
      },
    );
  }
}

class _Banner extends StatelessWidget {
  const _Banner({required this.icon, required this.color, required this.text, this.action});

  final IconData icon;
  final Color color;
  final String text;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.5)),
      ),
      child: Row(
        children: [
          Icon(icon, color: color),
          const SizedBox(width: 12),
          Expanded(child: Text(text)),
          if (action != null) ...[const SizedBox(width: 12), action!],
        ],
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  const _StatCard({required this.icon, required this.label, required this.value, required this.hint, this.live});

  final IconData icon;
  final String label;
  final String value;
  final String hint;
  final Widget? live;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    return PanelCard(
      title: label,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, color: colors.accent, size: 18),
          const SizedBox(height: 8),
          live ?? Text(value, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
          Text(hint, style: TextStyle(color: colors.muted, fontSize: 12), maxLines: 2, overflow: TextOverflow.ellipsis),
        ],
      ),
    );
  }
}

/// Ticks locally every second; the start time comes from the Rust supervisor.
class _Uptime extends StatefulWidget {
  const _Uptime({required this.startedUnix});

  final int? startedUnix;

  @override
  State<_Uptime> createState() => _UptimeState();
}

class _UptimeState extends State<_Uptime> {
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (widget.startedUnix != null) {
        setState(() {});
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final started = widget.startedUnix;
    final text = started == null ? '--' : formatDuration(DateTime.now().difference(DateTime.fromMillisecondsSinceEpoch(started * 1000)));
    return Text(text, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700));
  }
}

class _Meter extends StatelessWidget {
  const _Meter({required this.title, required this.value, required this.percent, required this.samples});

  final String title;
  final String value;
  final double percent;
  final List<double> samples;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
        Text(value, style: TextStyle(color: colors.muted, fontSize: 12)),
        const SizedBox(height: 8),
        ClipRRect(
          borderRadius: BorderRadius.circular(6),
          child: LinearProgressIndicator(value: percent.clamp(0, 1), minHeight: 6, backgroundColor: colors.track, color: colors.accent),
        ),
        const SizedBox(height: 8),
        Sparkline(samples: samples, color: colors.accent, idleColor: colors.cardBorder, height: 48),
      ],
    );
  }
}
