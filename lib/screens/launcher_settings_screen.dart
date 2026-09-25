// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';
import 'package:voxel_panel/widgets/common/section_header.dart';

class LauncherSettingsScreen extends StatelessWidget {
  const LauncherSettingsScreen({super.key, this.onClose});

  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final paths = appPaths();
    Widget folder(String label, String path) => Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: OutlinedButton.icon(
        onPressed: () => runGuarded(context, () => openInExplorer(path: path)),
        icon: const Icon(Icons.folder_open_outlined),
        label: Text(label),
      ),
    );
    return Scaffold(
      body: ListView(
        padding: const EdgeInsets.fromLTRB(28, 24, 28, 28),
        children: [
          SectionHeader(
            title: l.settings,
            subtitle: l.settingsSubtitle,
            leading: IconButton(onPressed: onClose ?? () => Navigator.pop(context), icon: const Icon(Icons.arrow_back)),
          ),
          const SizedBox(height: 20),
          ResponsiveGrid(
            minItemWidth: 340,
            children: [
              PanelCard(
                icon: Icons.folder_outlined,
                title: l.settingsLocalData,
                subtitle: l.settingsLocalDataSubtitle,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(paths.data, style: TextStyle(color: context.voxel.muted)),
                    const SizedBox(height: 12),
                    folder(l.openDataFolder, paths.data),
                    folder(l.openServersFolder, paths.servers),
                    folder(l.openRuntimesFolder, paths.runtimes),
                    folder(l.openBackupsFolder, paths.backups),
                  ],
                ),
              ),
              PanelCard(
                icon: Icons.info_outline,
                title: l.settingsAbout,
                subtitle: l.settingsAboutSubtitle,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(l.appTitle),
                    const SizedBox(height: 6),
                    Text(l.licenseLine, style: TextStyle(color: context.voxel.muted)),
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
