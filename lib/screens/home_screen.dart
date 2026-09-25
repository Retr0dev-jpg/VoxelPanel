// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/create_server_screen.dart';
import 'package:voxel_panel/screens/server/server_screen.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/section_header.dart';
import 'package:voxel_panel/widgets/delete_server_dialog.dart';
import 'package:voxel_panel/widgets/server_list_view.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  var _query = '';
  Set<String>? _selection;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final servers = ref.watch(serverListProvider);
    ref.watch(runtimeProvider);
    final selecting = _selection != null;
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(entries: [SidebarEntry(label: l.navServers, icon: Icons.dns_outlined)], selected: 0, onSelected: (_) {}),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(28, 24, 28, 8),
                  child: SectionHeader(
                    title: selecting ? l.selectedCount(_selection!.length) : l.navServers,
                    subtitle: selecting ? null : l.homeSubtitle,
                    actions: selecting ? _selectionActions(context, servers.value ?? const []) : _defaultActions(context),
                  ),
                ),
                Expanded(
                  child: servers.when(
                    data: (items) => _list(context, items),
                    error: (error, stack) => ErrorState(error: error, onRetry: _reload),
                    loading: () => const Center(child: CircularProgressIndicator()),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  List<Widget> _defaultActions(BuildContext context) {
    final l = context.l10n;
    return [
      SizedBox(
        width: 240,
        child: TextField(
          decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.searchServers, isDense: true),
          onChanged: (value) => setState(() => _query = value.trim().toLowerCase()),
        ),
      ),
      IconButton(tooltip: l.refresh, onPressed: _reload, icon: const Icon(Icons.refresh)),
      IconButton(tooltip: l.selectServers, onPressed: () => setState(() => _selection = {}), icon: const Icon(Icons.checklist)),
      OutlinedButton.icon(onPressed: () => _create(importing: true), icon: const Icon(Icons.download_outlined), label: Text(l.importServer)),
      FilledButton.icon(onPressed: _create, icon: const Icon(Icons.add), label: Text(l.newServer)),
    ];
  }

  List<Widget> _selectionActions(BuildContext context, List<ServerSummary> servers) {
    final l = context.l10n;
    final colors = context.voxel;
    final selected = servers.where((server) => _selection!.contains(server.id)).toList();
    final none = selected.isEmpty;
    return [
      TextButton(
        onPressed: () => setState(() => _selection = servers.map((server) => server.id).toSet()),
        child: Text(l.selectAll),
      ),
      FilledButton(
        style: actionButtonStyle(colors.start),
        onPressed: none ? null : () => _bulk(selected, (server) => startServer(id: server.id)),
        child: Text(l.actionStart),
      ),
      FilledButton(
        style: actionButtonStyle(colors.stop),
        onPressed: none ? null : () => _bulk(selected, (server) => stopServer(id: server.id)),
        child: Text(l.actionStop),
      ),
      OutlinedButton.icon(
        onPressed: none ? null : () => _deleteMany(selected),
        icon: const Icon(Icons.delete_outline),
        label: Text(l.delete),
      ),
      TextButton(onPressed: () => setState(() => _selection = null), child: Text(l.cancel)),
    ];
  }

  Widget _list(BuildContext context, List<ServerSummary> items) {
    final l = context.l10n;
    if (items.isEmpty) {
      return EmptyState(
        icon: Icons.dns_outlined,
        message: l.noServers,
        action: FilledButton.icon(onPressed: _create, icon: const Icon(Icons.add), label: Text(l.newServer)),
      );
    }
    final visible = items.where((server) {
      return _query.isEmpty || server.name.toLowerCase().contains(_query) || '${server.port}'.contains(_query);
    }).toList();
    if (visible.isEmpty) {
      return EmptyState(icon: Icons.search_off, message: l.noSearchResults);
    }
    return ServerListView(
      servers: visible,
      selection: _selection,
      onToggleSelected: (server) => setState(() {
        final selection = _selection!;
        selection.contains(server.id) ? selection.remove(server.id) : selection.add(server.id);
      }),
      onOpen: _open,
      onStart: (server) => runGuarded(context, () => startServer(id: server.id)),
      onStop: (server) => runGuarded(context, () => stopServer(id: server.id)),
      onRestart: (server) => runGuarded(context, () => restartServer(id: server.id)),
    );
  }

  Future<void> _reload() => ref.read(serverListProvider.notifier).reload();

  Future<void> _create({bool importing = false}) async {
    final created = await Navigator.push<bool>(
      context,
      MaterialPageRoute(builder: (context) => CreateServerScreen(startOnImport: importing)),
    );
    if (created == true) {
      await _reload();
    }
  }

  Future<void> _open(ServerSummary server) async {
    await Navigator.push(context, MaterialPageRoute(builder: (context) => ServerScreen(serverId: server.id)));
    await _reload();
  }

  Future<void> _bulk(List<ServerSummary> servers, Future<void> Function(ServerSummary server) action) async {
    for (final server in servers) {
      if (!mounted) {
        return;
      }
      await runGuarded(context, () => action(server));
    }
  }

  Future<void> _deleteMany(List<ServerSummary> servers) async {
    final options = await showDeleteServerDialog(context, names: servers.map((server) => server.name).toList());
    if (options == null || !mounted) {
      return;
    }
    for (final server in servers) {
      if (!mounted) {
        return;
      }
      await runGuarded(context, () => deleteServer(id: server.id, deleteFiles: options.deleteFiles, deleteBackups: options.deleteBackups));
    }
    setState(() => _selection = null);
    await _reload();
  }
}
