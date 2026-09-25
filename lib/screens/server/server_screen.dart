// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/server/backups_tab.dart';
import 'package:voxel_panel/screens/server/console_tab.dart';
import 'package:voxel_panel/screens/server/overview_tab.dart';
import 'package:voxel_panel/screens/server/plugins_tab.dart';
import 'package:voxel_panel/screens/server/properties_tab.dart';
import 'package:voxel_panel/screens/server/worlds_tab.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/providers.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/change_version_dialog.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/delete_server_dialog.dart';

class ServerScreen extends ConsumerStatefulWidget {
  const ServerScreen({super.key, required this.serverId});

  final String serverId;

  @override
  ConsumerState<ServerScreen> createState() => _ServerScreenState();
}

class _ServerScreenState extends ConsumerState<ServerScreen> {
  var _section = 0;

  void _refresh() => ref.invalidate(serverDetailsProvider(widget.serverId));

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final details = ref.watch(serverDetailsProvider(widget.serverId));
    final status = ref.watch(serverStatusProvider(widget.serverId));
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            onBack: () => Navigator.pop(context),
            selected: _section,
            onSelected: (index) => setState(() => _section = index),
            entries: [
              SidebarEntry(label: l.tabOverview, icon: Icons.space_dashboard_outlined),
              SidebarEntry(label: l.tabConsole, icon: Icons.terminal_outlined),
              SidebarEntry(label: l.tabProperties, icon: Icons.tune),
              SidebarEntry(label: l.tabPlugins, icon: Icons.extension_outlined),
              SidebarEntry(label: l.tabWorlds, icon: Icons.public_outlined),
              SidebarEntry(label: l.tabBackups, icon: Icons.inventory_2_outlined),
            ],
          ),
          Expanded(
            child: details.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (error, stack) => ErrorState(error: error, onRetry: _refresh),
              data: (details) => _body(context, details, status),
            ),
          ),
        ],
      ),
    );
  }

  Widget _body(BuildContext context, ServerDetails details, ServerStatus status) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (_section != 0)
          Padding(
            padding: const EdgeInsets.fromLTRB(28, 22, 28, 4),
            child: Text(details.name, style: const TextStyle(fontSize: 28, fontWeight: FontWeight.w700), overflow: TextOverflow.ellipsis),
          ),
        Expanded(
          child: IndexedStack(
            index: _section,
            children: [
              OverviewTab(
                details: details,
                onChanged: _refresh,
                onDelete: () => _delete(details),
                onRename: () => _rename(details),
                onChangeVersion: () async {
                  if (await showChangeVersionDialog(context, details)) {
                    _refresh();
                  }
                },
              ),
              ConsoleTab(serverId: widget.serverId, running: status == ServerStatus.running || status == ServerStatus.starting),
              PropertiesTab(serverId: widget.serverId),
              PluginsTab(serverId: widget.serverId, running: status.isActive),
              WorldsTab(serverId: widget.serverId, running: status.isActive),
              BackupsTab(serverId: widget.serverId, running: status.isActive),
            ],
          ),
        ),
      ],
    );
  }

  Future<void> _rename(ServerDetails details) async {
    final l = context.l10n;
    final controller = TextEditingController(text: details.name);
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l.renameServer),
        content: TextField(controller: controller, autofocus: true, onSubmitted: (value) => Navigator.pop(context, value)),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel)),
          FilledButton(onPressed: () => Navigator.pop(context, controller.text), child: Text(l.save)),
        ],
      ),
    );
    controller.dispose();
    if (name == null || name.trim() == details.name || !mounted) {
      return;
    }
    if (await runGuarded(context, () => renameServer(id: details.id, name: name.trim()))) {
      _refresh();
    }
  }

  Future<void> _delete(ServerDetails details) async {
    final options = await showDeleteServerDialog(context, names: [details.name]);
    if (options == null || !mounted) {
      return;
    }
    final deleted = await runGuarded(
      context,
      () => deleteServer(id: details.id, deleteFiles: options.deleteFiles, deleteBackups: options.deleteBackups),
    );
    if (deleted && mounted) {
      Navigator.pop(context);
    }
  }
}
