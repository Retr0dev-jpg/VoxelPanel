// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/theme.dart';

class LauncherSettingsScreen extends StatelessWidget {
  const LauncherSettingsScreen({super.key, this.onClose});

  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final data = appDataDir();
    return Scaffold(
      body: ListView(
        padding: const EdgeInsets.fromLTRB(28, 24, 28, 28),
        children: [
          Row(
            children: [
              IconButton(onPressed: onClose ?? () => Navigator.pop(context), icon: const Icon(Icons.arrow_back)),
              const SizedBox(width: 8),
              const Icon(Icons.settings_outlined, color: panelAccent),
              const SizedBox(width: 12),
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Impostazioni', style: TextStyle(fontSize: 28, fontWeight: FontWeight.w700)),
                    Text('Cartelle locali di VoxelPanel e informazioni sull\'applicazione.', style: TextStyle(color: panelMuted)),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              _SettingsCard(
                icon: Icons.folder_outlined,
                title: 'Dati locali',
                subtitle: 'Catalogo, server creati da VoxelPanel e backup.',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(data, style: const TextStyle(color: panelMuted)),
                    const SizedBox(height: 12),
                    OutlinedButton(onPressed: () => openInExplorer(path: data), child: const Text('Apri cartella dati')),
                    const SizedBox(height: 8),
                    OutlinedButton(onPressed: () => openInExplorer(path: '$data\\servers'), child: const Text('Apri cartella server')),
                    const SizedBox(height: 8),
                    OutlinedButton(onPressed: () => openInExplorer(path: '$data\\runtimes'), child: const Text('Apri runtime Java')),
                  ],
                ),
              ),
              const _SettingsCard(
                icon: Icons.info_outline,
                title: 'Informazioni',
                subtitle: 'Versione e licenza.',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('VoxelPanel 1.0.0'),
                    SizedBox(height: 6),
                    Text('Open Source · AGPL-3.0', style: TextStyle(color: panelMuted)),
                    SizedBox(height: 6),
                    Text('Gestione locale di server Paper.', style: TextStyle(color: panelMuted)),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  const _SettingsCard({required this.icon, required this.title, required this.subtitle, required this.child});

  final IconData icon;
  final String title;
  final String subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 420,
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(color: panelCard, borderRadius: BorderRadius.circular(16), border: Border.all(color: panelCardBorder)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, color: panelAccent, size: 18),
                const SizedBox(width: 8),
                Text(title, style: const TextStyle(fontWeight: FontWeight.w700)),
              ],
            ),
            const SizedBox(height: 4),
            Text(subtitle, style: const TextStyle(color: panelMuted, fontSize: 12)),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}
