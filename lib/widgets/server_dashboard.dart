// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';

class ServerDashboard extends StatefulWidget {
  const ServerDashboard({
    super.key,
    required this.details,
    required this.stats,
    required this.onAction,
  });

  final ServerDetails details;
  final ProcessStats? stats;
  final Future<bool> Function(Future<void> Function() action) onAction;

  @override
  State<ServerDashboard> createState() => _ServerDashboardState();
}

class _ServerDashboardState extends State<ServerDashboard> {
  final _cpu = <double>[];
  final _ram = <double>[];

  @override
  void didUpdateWidget(ServerDashboard oldWidget) {
    super.didUpdateWidget(oldWidget);
    final stats = widget.stats;
    if (stats == null) {
      return;
    }
    _cpu.add(stats.cpuPercent.clamp(0, 100));
    final maxBytes = _ramBytes(widget.details.ramMax);
    _ram.add(maxBytes == null || maxBytes == 0 ? 0 : (stats.memoryBytes / maxBytes * 100).clamp(0, 100));
    if (_cpu.length > 40) {
      _cpu.removeAt(0);
      _ram.removeAt(0);
    }
  }

  @override
  Widget build(BuildContext context) {
    final details = widget.details;
    final stats = widget.stats;
    final running = details.status == ServerStatus.running || details.status == ServerStatus.starting;
    final online = details.status == ServerStatus.running;
    final ramMax = _ramBytes(details.ramMax);
    final ramPercent = stats == null || ramMax == null || ramMax == 0 ? null : (stats.memoryBytes / ramMax * 100).clamp(0, 100);

    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 8, 28, 12),
      child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(14),
              child: Image.asset('assets/paper.png', width: 64, height: 64, fit: BoxFit.cover),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(details.name, style: const TextStyle(fontSize: 32, fontWeight: FontWeight.w700)),
                  const SizedBox(height: 4),
                  Row(
                    children: [
                      Icon(Icons.circle, size: 10, color: online ? panelOnline : panelMuted),
                      const SizedBox(width: 8),
                      Text(statusLabel(details.status), style: TextStyle(color: online ? panelOnline : panelMuted)),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Paper ${details.paperVersion ?? 'n/d'}   ·   Java ${details.javaMajor ?? 'n/d'}   ·   RAM ${details.ramMin} / ${details.ramMax}',
                    style: const TextStyle(color: panelMuted),
                  ),
                  Text(details.root, style: const TextStyle(color: panelMuted)),
                ],
              ),
            ),
            _actions(running),
          ],
        ),
        const SizedBox(height: 20),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            _StatCard(icon: Icons.people_outline, label: 'Giocatori', value: '${details.onlinePlayers} / ${details.maxPlayers}', hint: details.onlinePlayers == 0 ? 'Nessun giocatore online' : 'Giocatori collegati'),
            _StatCard(icon: Icons.memory, label: 'CPU', value: stats == null ? '--' : '${stats.cpuPercent.toStringAsFixed(0)}%', hint: 'Utilizzo del processo'),
            _StatCard(icon: Icons.storage_outlined, label: 'Memoria', value: stats == null ? '--' : formatBytes(stats.memoryBytes), hint: 'di ${details.ramMax}'),
            const _StatCard(icon: Icons.show_chart, label: 'TPS', value: '--', hint: 'Non disponibile'),
            const _StatCard(icon: Icons.schedule, label: 'Uptime', value: '--', hint: 'Non disponibile'),
          ],
        ),
        const SizedBox(height: 16),
        _Panel(
          title: 'Statistiche live',
          subtitle: 'Dati letti dal processo del server. Disco, rete e TPS non sono misurati.',
          trailing: Text(online ? 'Server online' : 'Server offline', style: TextStyle(color: online ? panelOnline : const Color(0xFFFF6B6B))),
          child: Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _Meter(title: 'CPU', value: stats == null ? '--' : '${stats.cpuPercent.toStringAsFixed(0)}%', percent: stats == null ? 0 : stats.cpuPercent.clamp(0, 100) / 100, samples: _cpu),
              _Meter(title: 'Memoria RAM', value: stats == null ? '${details.ramMin} / ${details.ramMax}' : '${formatBytes(stats.memoryBytes)} / ${details.ramMax}', percent: (ramPercent ?? 0) / 100, samples: _ram),
              const _Meter(title: 'Disco', value: '--', percent: 0, samples: []),
              const _Meter(title: 'Rete in download', value: '--', percent: 0, samples: []),
              const _Meter(title: 'Rete in upload', value: '--', percent: 0, samples: []),
              const _Meter(title: 'TPS', value: '--', percent: 0, samples: []),
            ],
          ),
        ),
      ],
      ),
    );
  }

  ButtonStyle _actionStyle(Color color) {
    return FilledButton.styleFrom(backgroundColor: color, foregroundColor: Colors.white);
  }

  Widget _actions(bool running) {
    final details = widget.details;
    return Wrap(
      spacing: 8,
      children: [
        FilledButton.icon(
          style: _actionStyle(const Color(0xFF2E7D32)),
          onPressed: running ? null : () => widget.onAction(() => startServer(id: details.id)),
          icon: const Icon(Icons.play_arrow),
          label: const Text('Avvia'),
        ),
        FilledButton(
          style: _actionStyle(const Color(0xFFC62828)),
          onPressed: running ? () => widget.onAction(() => stopServer(id: details.id)) : null,
          child: const Text('Stop'),
        ),
        FilledButton(
          style: _actionStyle(const Color(0xFFEF6C00)),
          onPressed: running ? () => widget.onAction(() => restartServer(id: details.id)) : null,
          child: const Text('Riavvia'),
        ),
        OutlinedButton.icon(onPressed: () => widget.onAction(() => openInExplorer(path: details.root)), icon: const Icon(Icons.folder_open_outlined), label: const Text('Apri cartella')),
      ],
    );
  }
}

double? _ramBytes(String raw) {
  final match = RegExp(r'^(\d+(?:\.\d+)?)([KMG])$', caseSensitive: false).firstMatch(raw.trim());
  if (match == null) {
    return null;
  }
  final number = double.parse(match.group(1)!);
  final unit = match.group(2)!.toUpperCase();
  final multiplier = switch (unit) {
    'K' => 1024,
    'M' => 1024 * 1024,
    'G' => 1024 * 1024 * 1024,
    _ => 1,
  };
  return number * multiplier;
}

class _StatCard extends StatelessWidget {
  const _StatCard({required this.icon, required this.label, required this.value, required this.hint});

  final IconData icon;
  final String label;
  final String value;
  final String hint;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 180,
      child: _Panel(
        title: label,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: panelAccent, size: 18),
            const SizedBox(height: 8),
            Text(value, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
            Text(hint, style: const TextStyle(color: panelMuted, fontSize: 12)),
          ],
        ),
      ),
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({required this.title, required this.child, this.subtitle, this.trailing});

  final String title;
  final String? subtitle;
  final Widget? trailing;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(color: panelCard, borderRadius: BorderRadius.circular(16), border: Border.all(color: panelCardBorder)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(child: Text(title, style: const TextStyle(fontWeight: FontWeight.w700))),
              if (trailing != null) trailing!,
            ],
          ),
          if (subtitle != null) Text(subtitle!, style: const TextStyle(color: panelMuted, fontSize: 12)),
          const SizedBox(height: 12),
          child,
        ],
      ),
    );
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
    return SizedBox(
      width: 280,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
          Text(value, style: const TextStyle(color: panelMuted, fontSize: 12)),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(6),
            child: LinearProgressIndicator(value: percent.clamp(0, 1), minHeight: 6, backgroundColor: const Color(0xFF2C2A3A), color: panelAccent),
          ),
          const SizedBox(height: 8),
          SizedBox(height: 36, width: double.infinity, child: CustomPaint(painter: _SparkPainter(samples))),
        ],
      ),
    );
  }
}

class _SparkPainter extends CustomPainter {
  _SparkPainter(this.samples);

  final List<double> samples;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = panelAccent
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;
    if (samples.length < 2) {
      canvas.drawLine(Offset(0, size.height / 2), Offset(size.width, size.height / 2), paint..color = panelCardBorder);
      return;
    }
    final path = Path();
    for (var i = 0; i < samples.length; i++) {
      final x = size.width * i / (samples.length - 1);
      final y = size.height - (samples[i].clamp(0, 100) / 100) * size.height;
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant _SparkPainter oldDelegate) => oldDelegate.samples != samples;
}
