// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/theme.dart';

const compactSidebarBreakpoint = 1000.0;

class SidebarEntry {
  const SidebarEntry({required this.label, required this.icon});

  final String label;
  final IconData icon;
}

class AppSidebar extends StatelessWidget {
  const AppSidebar({super.key, required this.entries, required this.selected, required this.onSelected, this.onBack, this.backLabel});

  final List<SidebarEntry> entries;
  final int selected;
  final ValueChanged<int> onSelected;
  final VoidCallback? onBack;
  final String? backLabel;

  @override
  Widget build(BuildContext context) {
    final compact = MediaQuery.sizeOf(context).width < compactSidebarBreakpoint;
    final colors = context.voxel;
    final l = context.l10n;
    return AnimatedContainer(
      duration: const Duration(milliseconds: 180),
      width: compact ? 76 : 248,
      color: colors.sidebar,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: EdgeInsets.fromLTRB(compact ? 20 : 20, 22, compact ? 20 : 20, 18),
            child: Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(color: colors.accent, borderRadius: BorderRadius.circular(10)),
                  child: const Icon(Icons.view_in_ar, color: Colors.white, size: 20),
                ),
                if (!compact) ...[
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(l.appTitle, style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
                        Text(l.appTagline, style: TextStyle(color: colors.muted, fontSize: 12), overflow: TextOverflow.ellipsis),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (onBack != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
              child: compact
                  ? IconButton(tooltip: backLabel ?? l.allServers, onPressed: onBack, icon: const Icon(Icons.arrow_back))
                  : TextButton.icon(onPressed: onBack, icon: const Icon(Icons.arrow_back, size: 18), label: Text(backLabel ?? l.allServers)),
            ),
          Expanded(
            child: ListView(
              padding: EdgeInsets.zero,
              children: [
                for (var index = 0; index < entries.length; index++)
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
                    child: _NavTile(entry: entries[index], selected: index == selected, compact: compact, onTap: () => onSelected(index)),
                  ),
              ],
            ),
          ),
          if (!compact)
            Padding(
              padding: const EdgeInsets.all(20),
              child: Text(l.licenseLine, style: TextStyle(color: colors.muted, fontSize: 12)),
            ),
        ],
      ),
    );
  }
}

class _NavTile extends StatelessWidget {
  const _NavTile({required this.entry, required this.selected, required this.compact, required this.onTap});

  final SidebarEntry entry;
  final bool selected;
  final bool compact;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    final foreground = selected ? Theme.of(context).colorScheme.onSurface : colors.muted;
    final tile = Material(
      color: selected ? colors.accent.withValues(alpha: 0.22) : Colors.transparent,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
          child: Row(
            mainAxisAlignment: compact ? MainAxisAlignment.center : MainAxisAlignment.start,
            children: [
              Icon(entry.icon, size: 20, color: foreground),
              if (!compact) ...[
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    entry.label,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: foreground, fontWeight: selected ? FontWeight.w600 : FontWeight.w500),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
    return compact ? Tooltip(message: entry.label, child: tile) : tile;
  }
}
