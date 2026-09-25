// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:voxel_panel/src/rust/api/types.dart';

String statusLabel(ServerStatus status) {
  return switch (status) {
    ServerStatus.stopped => 'Fermo',
    ServerStatus.starting => 'Avvio',
    ServerStatus.running => 'Online',
    ServerStatus.stopping => 'Arresto',
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

String readableError(Object error) {
  return error.toString().replaceFirst(RegExp(r'^[A-Za-z0-9_]+Exception: '), '');
}
