// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:window_manager/window_manager.dart';

class WindowTitleBar extends StatelessWidget {
  const WindowTitleBar({super.key, required this.onSettings});

  final VoidCallback onSettings;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    final l = context.l10n;
    return Material(
      color: colors.sidebar,
      child: SizedBox(
        height: 40,
        width: MediaQuery.sizeOf(context).width,
        child: Row(
          children: [
            Expanded(
              child: DragToMoveArea(
                child: Padding(
                  padding: const EdgeInsets.only(left: 16),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: Text(l.appTitle, style: TextStyle(color: colors.muted, fontSize: 13)),
                  ),
                ),
              ),
            ),
            IconButton(tooltip: l.settings, onPressed: onSettings, icon: const Icon(Icons.settings_outlined, size: 18)),
            _CaptionButton(icon: Icons.remove, tooltip: l.windowMinimize, onPressed: windowManager.minimize),
            _CaptionButton(
              icon: Icons.crop_square,
              tooltip: l.windowMaximize,
              onPressed: () async {
                if (await windowManager.isMaximized()) {
                  await windowManager.unmaximize();
                } else {
                  await windowManager.maximize();
                }
              },
            ),
            _CaptionButton(icon: Icons.close, tooltip: l.windowClose, onPressed: windowManager.close),
          ],
        ),
      ),
    );
  }
}

class _CaptionButton extends StatelessWidget {
  const _CaptionButton({required this.icon, required this.tooltip, required this.onPressed});

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 46,
      height: 40,
      child: IconButton(tooltip: tooltip, onPressed: onPressed, icon: Icon(icon, size: 16)),
    );
  }
}
