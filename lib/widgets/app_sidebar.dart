// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/theme.dart';

class SidebarEntry {
  const SidebarEntry({required this.label, required this.icon});

  final String label;
  final IconData icon;
}

class AppSidebar extends StatelessWidget {
  const AppSidebar({super.key, required this.entries, required this.selected, required this.onSelected, this.onBack});

  final List<SidebarEntry> entries;
  final int selected;
  final ValueChanged<int> onSelected;
  final VoidCallback? onBack;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 248,
      color: panelSidebar,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 22, 20, 18),
            child: Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(color: panelAccent, borderRadius: BorderRadius.circular(10)),
                  child: const Icon(Icons.view_in_ar, color: Colors.white, size: 20),
                ),
                const SizedBox(width: 12),
                const Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('VoxelPanel', style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
                      Text('Server Paper in locale', style: TextStyle(color: panelMuted, fontSize: 12)),
                    ],
                  ),
                ),
              ],
            ),
          ),
          if (onBack != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
              child: TextButton.icon(
                onPressed: onBack,
                icon: const Icon(Icons.arrow_back, size: 18),
                label: const Text('Tutti i server'),
              ),
            ),
          for (var index = 0; index < entries.length; index++)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
              child: _NavTile(entry: entries[index], selected: index == selected, onTap: () => onSelected(index)),
            ),
          const Spacer(),
          const Padding(
            padding: EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('VoxelPanel v1.0.0', style: TextStyle(fontWeight: FontWeight.w600)),
                SizedBox(height: 2),
                Text('Open Source · AGPL-3.0', style: TextStyle(color: panelMuted, fontSize: 12)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _NavTile extends StatelessWidget {
  const _NavTile({required this.entry, required this.selected, required this.onTap});

  final SidebarEntry entry;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: selected ? panelAccent.withValues(alpha: 0.22) : Colors.transparent,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
          child: Row(
            children: [
              Icon(entry.icon, size: 20, color: selected ? Colors.white : panelMuted),
              const SizedBox(width: 12),
              Text(entry.label, style: TextStyle(color: selected ? Colors.white : panelMuted, fontWeight: selected ? FontWeight.w600 : FontWeight.w500)),
            ],
          ),
        ),
      ),
    );
  }
}
