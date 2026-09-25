// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/theme.dart';

class PanelCard extends StatelessWidget {
  const PanelCard({super.key, required this.child, this.title, this.subtitle, this.icon, this.trailing, this.padding = const EdgeInsets.all(16)});

  final Widget child;
  final String? title;
  final String? subtitle;
  final IconData? icon;
  final Widget? trailing;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    // Material (not DecoratedBox) so ListTiles inside keep their ink and background.
    return Material(
      color: colors.card,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: colors.cardBorder),
      ),
      child: Padding(
        padding: padding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (title != null) ...[
              Row(
                children: [
                  if (icon != null) ...[Icon(icon, color: colors.accent, size: 18), const SizedBox(width: 8)],
                  Expanded(
                    child: Text(title!, style: const TextStyle(fontWeight: FontWeight.w700)),
                  ),
                  ?trailing,
                ],
              ),
              if (subtitle != null) Text(subtitle!, style: TextStyle(color: colors.muted, fontSize: 12)),
              const SizedBox(height: 12),
            ],
            child,
          ],
        ),
      ),
    );
  }
}

/// Lays children out in equal-width columns that adapt to the available width. Items in the same
/// row share the tallest height, so children must support intrinsic sizing (no LayoutBuilder).
class ResponsiveGrid extends StatelessWidget {
  const ResponsiveGrid({super.key, required this.children, this.minItemWidth = 220, this.spacing = 12});

  final List<Widget> children;
  final double minItemWidth;
  final double spacing;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = ((constraints.maxWidth + spacing) / (minItemWidth + spacing)).floor().clamp(1, children.isEmpty ? 1 : children.length);
        final width = (constraints.maxWidth - spacing * (columns - 1)) / columns;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (var start = 0; start < children.length; start += columns) ...[
              if (start > 0) SizedBox(height: spacing),
              IntrinsicHeight(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (final (index, child) in children.skip(start).take(columns).indexed) ...[
                      if (index > 0) SizedBox(width: spacing),
                      SizedBox(width: width, child: child),
                    ],
                  ],
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}
