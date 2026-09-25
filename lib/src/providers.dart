// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

const statsHistoryLength = 60;

final serverListProvider = AsyncNotifierProvider<ServerListNotifier, List<ServerSummary>>(ServerListNotifier.new);

class ServerListNotifier extends AsyncNotifier<List<ServerSummary>> {
  @override
  Future<List<ServerSummary>> build() => listServers();

  Future<void> reload() async {
    state = await AsyncValue.guard(listServers);
  }
}

final serverDetailsProvider = FutureProvider.autoDispose.family<ServerDetails, String>((ref, id) => getServer(id: id));

class StatSample {
  const StatSample(this.cpuPercent, this.memoryBytes);

  final double cpuPercent;
  final int memoryBytes;
}

class RuntimeState {
  const RuntimeState({this.servers = const {}, this.history = const {}});

  final Map<String, ServerRuntime> servers;
  final Map<String, List<StatSample>> history;

  ServerStatus statusOf(String id) => servers[id]?.status ?? ServerStatus.stopped;
}

/// Mirrors the Rust supervisor. Updates arrive through `watchEvents`, so nothing polls.
final runtimeProvider = NotifierProvider<RuntimeNotifier, RuntimeState>(RuntimeNotifier.new);

class RuntimeNotifier extends Notifier<RuntimeState> {
  StreamSubscription<ServerRuntime>? _subscription;

  @override
  RuntimeState build() {
    _subscription = watchEvents().listen(_apply);
    ref.onDispose(() => _subscription?.cancel());
    return const RuntimeState();
  }

  void _apply(ServerRuntime runtime) {
    final history = Map<String, List<StatSample>>.of(state.history);
    if (runtime.pid == null) {
      history.remove(runtime.serverId);
    } else {
      final previous = state.servers[runtime.serverId];
      final changed = previous == null || previous.cpuPercent != runtime.cpuPercent || previous.memoryBytes != runtime.memoryBytes;
      if (changed) {
        final samples = [...?history[runtime.serverId], StatSample(runtime.cpuPercent, runtime.memoryBytes)];
        history[runtime.serverId] = samples.length > statsHistoryLength ? samples.sublist(samples.length - statsHistoryLength) : samples;
      }
    }
    state = RuntimeState(servers: {...state.servers, runtime.serverId: runtime}, history: history);
  }
}

final serverRuntimeProvider = Provider.family<ServerRuntime?, String>((ref, id) => ref.watch(runtimeProvider.select((state) => state.servers[id])));

final serverStatusProvider = Provider.family<ServerStatus, String>(
  (ref, id) => ref.watch(runtimeProvider.select((state) => state.statusOf(id))),
);

final statsHistoryProvider = Provider.family<List<StatSample>, String>(
  (ref, id) => ref.watch(runtimeProvider.select((state) => state.history[id] ?? const <StatSample>[])),
);

extension ServerStatusX on ServerStatus {
  bool get isActive => this != ServerStatus.stopped;
  bool get isOnline => this == ServerStatus.running;
}
