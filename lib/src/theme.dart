// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';

const defaultAccent = Color(0xFF7C4DFF);

/// Semantic colors used across the panel; widgets must read them from the theme.
@immutable
class VoxelColors extends ThemeExtension<VoxelColors> {
  const VoxelColors({
    required this.background,
    required this.sidebar,
    required this.card,
    required this.cardBorder,
    required this.field,
    required this.muted,
    required this.accent,
    required this.online,
    required this.start,
    required this.stop,
    required this.restart,
    required this.danger,
    required this.warning,
    required this.console,
    required this.track,
  });

  factory VoxelColors.dark(Color accent) => VoxelColors(
    background: const Color(0xFF16141F),
    sidebar: const Color(0xFF1C1A27),
    card: const Color(0xFF242232),
    cardBorder: const Color(0xFF343246),
    field: const Color(0xFF201E2C),
    muted: const Color(0xFF9A96AD),
    accent: accent,
    online: const Color(0xFF3DDC97),
    start: const Color(0xFF2E7D32),
    stop: const Color(0xFFC62828),
    restart: const Color(0xFFEF6C00),
    danger: const Color(0xFFFF6B6B),
    warning: const Color(0xFFFFB74D),
    console: const Color(0xFF0F0E16),
    track: const Color(0xFF2C2A3A),
  );

  factory VoxelColors.light(Color accent) => VoxelColors(
    background: const Color(0xFFF6F5FA),
    sidebar: const Color(0xFFECEAF3),
    card: Colors.white,
    cardBorder: const Color(0xFFDDDAE8),
    field: const Color(0xFFF1EFF7),
    muted: const Color(0xFF6B6780),
    accent: accent,
    online: const Color(0xFF1E9E68),
    start: const Color(0xFF2E7D32),
    stop: const Color(0xFFC62828),
    restart: const Color(0xFFE65100),
    danger: const Color(0xFFD32F2F),
    warning: const Color(0xFFB26A00),
    console: const Color(0xFF1B1A24),
    track: const Color(0xFFE2DFEC),
  );

  final Color background;
  final Color sidebar;
  final Color card;
  final Color cardBorder;
  final Color field;
  final Color muted;
  final Color accent;
  final Color online;
  final Color start;
  final Color stop;
  final Color restart;
  final Color danger;
  final Color warning;
  final Color console;
  final Color track;

  @override
  VoxelColors copyWith({Color? accent}) => VoxelColors(
    background: background,
    sidebar: sidebar,
    card: card,
    cardBorder: cardBorder,
    field: field,
    muted: muted,
    accent: accent ?? this.accent,
    online: online,
    start: start,
    stop: stop,
    restart: restart,
    danger: danger,
    warning: warning,
    console: console,
    track: track,
  );

  @override
  VoxelColors lerp(ThemeExtension<VoxelColors>? other, double t) {
    if (other is! VoxelColors) {
      return this;
    }
    Color mix(Color a, Color b) => Color.lerp(a, b, t)!;
    return VoxelColors(
      background: mix(background, other.background),
      sidebar: mix(sidebar, other.sidebar),
      card: mix(card, other.card),
      cardBorder: mix(cardBorder, other.cardBorder),
      field: mix(field, other.field),
      muted: mix(muted, other.muted),
      accent: mix(accent, other.accent),
      online: mix(online, other.online),
      start: mix(start, other.start),
      stop: mix(stop, other.stop),
      restart: mix(restart, other.restart),
      danger: mix(danger, other.danger),
      warning: mix(warning, other.warning),
      console: mix(console, other.console),
      track: mix(track, other.track),
    );
  }
}

extension VoxelThemeContext on BuildContext {
  VoxelColors get voxel => Theme.of(this).extension<VoxelColors>()!;
}

ThemeData voxelTheme({Brightness brightness = Brightness.dark, Color accent = defaultAccent, VisualDensity? density}) {
  final dark = brightness == Brightness.dark;
  final colors = dark ? VoxelColors.dark(accent) : VoxelColors.light(accent);
  final foreground = dark ? Colors.white : const Color(0xFF1B1A24);
  final scheme = ColorScheme.fromSeed(seedColor: accent, brightness: brightness).copyWith(
    primary: accent,
    onPrimary: Colors.white,
    surface: colors.card,
    onSurface: foreground,
    error: colors.danger,
  );
  final radius = BorderRadius.circular(12);
  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    visualDensity: density,
    extensions: [colors],
    scaffoldBackgroundColor: colors.background,
    canvasColor: colors.background,
    dividerColor: colors.cardBorder,
    appBarTheme: AppBarTheme(backgroundColor: colors.background, foregroundColor: foreground, elevation: 0, scrolledUnderElevation: 0),
    cardTheme: CardThemeData(
      color: colors.card,
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16), side: BorderSide(color: colors.cardBorder)),
    ),
    dialogTheme: DialogThemeData(backgroundColor: colors.card),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: colors.field,
      border: OutlineInputBorder(borderRadius: radius, borderSide: BorderSide(color: colors.cardBorder)),
      enabledBorder: OutlineInputBorder(borderRadius: radius, borderSide: BorderSide(color: colors.cardBorder)),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(backgroundColor: accent, foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: radius)),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(foregroundColor: foreground, side: BorderSide(color: colors.cardBorder), shape: RoundedRectangleBorder(borderRadius: radius)),
    ),
    navigationRailTheme: NavigationRailThemeData(backgroundColor: colors.sidebar),
    snackBarTheme: const SnackBarThemeData(behavior: SnackBarBehavior.floating),
  );
}

/// Filled button colored with one of the semantic action colors.
ButtonStyle actionButtonStyle(Color color) => FilledButton.styleFrom(backgroundColor: color, foregroundColor: Colors.white);
