// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

/// Asset id of a provider; matches the Rust `id_of` and the file names in `assets/providers/`.
String providerId(ProviderKind kind) => switch (kind) {
  ProviderKind.neoForge => 'neoforge',
  ProviderKind.bungeeCord => 'bungeecord',
  ProviderKind.spongeVanilla => 'spongevanilla',
  ProviderKind.spongeForge => 'spongeforge',
  _ => kind.name,
};

String providerName(ProviderKind kind) {
  try {
    return providerInfo(kind: kind).name;
  } catch (_) {
    return kind.name;
  }
}

String categoryLabel(AppLocalizations l, ProviderCategory category) => switch (category) {
  ProviderCategory.vanilla => l.categoryVanilla,
  ProviderCategory.plugins => l.categoryPlugins,
  ProviderCategory.modded => l.categoryModded,
  ProviderCategory.proxy => l.categoryProxy,
  ProviderCategory.hybrid => l.categoryHybrid,
  ProviderCategory.other => l.categoryOther,
};

class ProviderIcon extends StatelessWidget {
  const ProviderIcon(this.kind, {super.key, this.size = 52});

  final ProviderKind kind;
  final double size;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(size * 0.22),
      child: Image.asset(
        'assets/providers/${providerId(kind)}.png',
        width: size,
        height: size,
        fit: BoxFit.cover,
        errorBuilder: (context, error, stack) => Image.asset('assets/providers/custom.png', width: size, height: size),
      ),
    );
  }
}
