// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/settings/launcher_settings_screen.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/settings.dart';

/// Layout of plugin, mod and catalogue lists, saved in the launcher settings.
ContentLayout watchContentLayout(WidgetRef ref) => ref.watch(settingsValueProvider)?.appearance.contentLayout ?? ContentLayout.list;

/// Fixed-height cells keep every card the same size whatever its text.
const contentGridDelegate = SliverGridDelegateWithMaxCrossAxisExtent(maxCrossAxisExtent: 260, mainAxisExtent: 196, crossAxisSpacing: 10, mainAxisSpacing: 10);

class ContentLayoutToggle extends ConsumerWidget {
  const ContentLayoutToggle({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final settings = ref.watch(settingsValueProvider);
    return SegmentedButton<ContentLayout>(
      showSelectedIcon: false,
      style: const ButtonStyle(visualDensity: VisualDensity.compact),
      segments: [
        ButtonSegment(value: ContentLayout.list, icon: const Icon(Icons.view_agenda_outlined), tooltip: l.layoutList),
        ButtonSegment(value: ContentLayout.grid, icon: const Icon(Icons.grid_view), tooltip: l.layoutGrid),
      ],
      selected: {watchContentLayout(ref)},
      onSelectionChanged: settings == null
          ? null
          : (selection) => saveSettings(context, ref, settings.copyWith(appearance: settings.appearance.copyWith(contentLayout: selection.first))),
    );
  }
}
