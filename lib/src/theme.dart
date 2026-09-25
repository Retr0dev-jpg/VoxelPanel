// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';

const panelBackground = Color(0xFF16141F);
const panelSidebar = Color(0xFF1C1A27);
const panelCard = Color(0xFF242232);
const panelCardBorder = Color(0xFF343246);
const panelAccent = Color(0xFF7C4DFF);
const panelOnline = Color(0xFF3DDC97);
const panelMuted = Color(0xFF9A96AD);

ThemeData voxelTheme() {
  final scheme = ColorScheme.fromSeed(seedColor: panelAccent, brightness: Brightness.dark).copyWith(
    primary: panelAccent,
    surface: panelCard,
    onSurface: Colors.white,
  );
  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: panelBackground,
    canvasColor: panelBackground,
    dividerColor: panelCardBorder,
    appBarTheme: const AppBarTheme(
      backgroundColor: panelBackground,
      foregroundColor: Colors.white,
      elevation: 0,
      scrolledUnderElevation: 0,
    ),
    cardTheme: CardThemeData(
      color: panelCard,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: const BorderSide(color: panelCardBorder),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: const Color(0xFF201E2C),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: const BorderSide(color: panelCardBorder)),
      enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(12), borderSide: const BorderSide(color: panelCardBorder)),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: panelAccent,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: Colors.white,
        side: const BorderSide(color: panelCardBorder),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    navigationRailTheme: const NavigationRailThemeData(backgroundColor: panelSidebar),
    snackBarTheme: const SnackBarThemeData(behavior: SnackBarBehavior.floating),
  );
}
