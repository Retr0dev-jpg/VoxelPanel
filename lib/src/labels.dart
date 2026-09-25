// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/widgets.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/error.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

String statusLabel(BuildContext context, ServerStatus status) {
  final l = context.l10n;
  return switch (status) {
    ServerStatus.stopped => l.statusStopped,
    ServerStatus.starting => l.statusStarting,
    ServerStatus.running => l.statusRunning,
    ServerStatus.stopping => l.statusStopping,
  };
}

String formatBytes(int bytes) {
  if (bytes < 1024) {
    return '$bytes B';
  }
  if (bytes < 1024 * 1024) {
    return '${(bytes / 1024).toStringAsFixed(1)} KB';
  }
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}

String formatDuration(Duration duration) {
  final days = duration.inDays;
  final hours = duration.inHours.remainder(24);
  final minutes = duration.inMinutes.remainder(60);
  final seconds = duration.inSeconds.remainder(60);
  if (days > 0) {
    return '${days}d ${hours}h ${minutes}m';
  }
  if (hours > 0) {
    return '${hours}h ${minutes}m';
  }
  if (minutes > 0) {
    return '${minutes}m ${seconds}s';
  }
  return '${seconds}s';
}

/// Parses JVM memory values such as `4G` or `2048M` into bytes.
double? memoryBytes(String raw) {
  final match = RegExp(r'^(\d+(?:\.\d+)?)([KMG])$', caseSensitive: false).firstMatch(raw.trim());
  if (match == null) {
    return null;
  }
  final number = double.parse(match.group(1)!);
  final multiplier = switch (match.group(2)!.toUpperCase()) {
    'K' => 1024,
    'M' => 1024 * 1024,
    'G' => 1024 * 1024 * 1024,
    _ => 1,
  };
  return number * multiplier;
}

/// Localized text for errors coming from Rust; unknown errors keep their own message.
String describeError(BuildContext context, Object error) {
  final l = context.l10n;
  if (error is PanelError) {
    return switch (error.code) {
      ErrorCode.serverRunning => l.errorServerRunning,
      ErrorCode.serverStopped => l.errorServerStopped,
      ErrorCode.eulaRequired => l.errorEulaRequired,
      ErrorCode.network => l.errorNetwork(error.message),
      _ => error.message,
    };
  }
  return error.toString().replaceFirst(RegExp(r'^[A-Za-z0-9_]+(Exception|Error): '), '');
}
