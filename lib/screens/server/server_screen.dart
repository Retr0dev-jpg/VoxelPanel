// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:voxel_panel/screens/server/addons_tab.dart';
import 'package:voxel_panel/screens/server/automation_tab.dart';
import 'package:voxel_panel/screens/server/backups_tab.dart';
import 'package:voxel_panel/screens/server/console_tab.dart';
import 'package:voxel_panel/screens/server/files_tab.dart';
import 'package:voxel_panel/screens/server/logs_tab.dart';
import 'package:voxel_panel/screens/server/overview_tab.dart';
import 'package:voxel_panel/screens/server/players_tab.dart';
import 'package:voxel_panel/screens/server/properties_tab.dart';
import 'package:voxel_panel/screens/server/server_settings_tab.dart';
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

enum ServerSection { overview, console, players, settings, automation, properties, files, plugins, mods, worlds, backups, logs }

/// Sections that make sense for a server type: proxies have no worlds or server.properties,
/// mod loaders get a mods section instead of (or next to) plugins.
List<ServerSection> sectionsFor(ProviderInfo info) => [
  ServerSection.overview,
  ServerSection.console,
  if (!info.isProxy) ServerSection.players,
  ServerSection.settings,
  ServerSection.automation,
  if (!info.isProxy) ServerSection.properties,
  ServerSection.files,
  if (info.supportsPlugins) ServerSection.plugins,
  if (info.supportsMods) ServerSection.mods,
  if (info.hasWorlds) ServerSection.worlds,
  ServerSection.backups,
  ServerSection.logs,
];

class _ServerScreenState extends ConsumerState<ServerScreen> {
  var _section = ServerSection.overview;

  void _refresh() => ref.invalidate(serverDetailsProvider(widget.serverId));

  SidebarEntry _entry(AppLocalizations l, ServerSection section) => switch (section) {
    ServerSection.overview => SidebarEntry(label: l.tabOverview, icon: Icons.space_dashboard_outlined),
    ServerSection.console => SidebarEntry(label: l.tabConsole, icon: Icons.terminal_outlined),
    ServerSection.players => SidebarEntry(label: l.tabPlayers, icon: Icons.people_outline),
    ServerSection.settings => SidebarEntry(label: l.tabSettings, icon: Icons.settings_applications_outlined),
    ServerSection.files => SidebarEntry(label: l.tabFiles, icon: Icons.folder_outlined),
    ServerSection.automation => SidebarEntry(label: l.tabAutomation, icon: Icons.event_repeat),
    ServerSection.logs => SidebarEntry(label: l.tabLogs, icon: Icons.receipt_long_outlined),
    ServerSection.properties => SidebarEntry(label: l.tabProperties, icon: Icons.tune),
    ServerSection.plugins => SidebarEntry(label: l.tabPlugins, icon: Icons.extension_outlined),
    ServerSection.mods => SidebarEntry(label: l.tabMods, icon: Icons.widgets_outlined),
    ServerSection.worlds => SidebarEntry(label: l.tabWorlds, icon: Icons.public_outlined),
    ServerSection.backups => SidebarEntry(label: l.tabBackups, icon: Icons.inventory_2_outlined),
  };

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final details = ref.watch(serverDetailsProvider(widget.serverId));
    final status = ref.watch(serverStatusProvider(widget.serverId));
    final sections = details.value == null ? const [ServerSection.overview] : sectionsFor(providerInfo(kind: details.value!.provider));
    final selected = sections.contains(_section) ? _section : ServerSection.overview;
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            onBack: () => Navigator.pop(context),
            selected: sections.indexOf(selected),
            onSelected: (index) => setState(() => _section = sections[index]),
            entries: [for (final section in sections) _entry(l, section)],
          ),
          Expanded(
            child: details.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (error, stack) => ErrorState(error: error, onRetry: _refresh),
              data: (details) => _body(context, details, status, sections, selected),
            ),
          ),
        ],
      ),
    );
  }

  Widget _body(BuildContext context, ServerDetails details, ServerStatus status, List<ServerSection> sections, ServerSection selected) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (selected != ServerSection.overview)
          Padding(
            padding: const EdgeInsets.fromLTRB(28, 22, 28, 4),
            child: Text(details.name, style: const TextStyle(fontSize: 28, fontWeight: FontWeight.w700), overflow: TextOverflow.ellipsis),
          ),
        Expanded(
          child: IndexedStack(
            index: sections.indexOf(selected),
            children: [
              for (final section in sections) _page(section, details, status),
            ],
          ),
        ),
      ],
    );
  }

  Widget _page(ServerSection section, ServerDetails details, ServerStatus status) {
    return switch (section) {
      ServerSection.overview => OverviewTab(
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
      ServerSection.console => ConsoleTab(serverId: widget.serverId, running: status == ServerStatus.running || status == ServerStatus.starting),
      ServerSection.players => PlayersTab(serverId: widget.serverId, running: status == ServerStatus.running),
      ServerSection.settings => ServerSettingsTab(
        details: details,
        running: status.isActive,
        onChanged: _refresh,
        onChangeVersion: () async {
          if (await showChangeVersionDialog(context, details)) {
            _refresh();
          }
        },
      ),
      ServerSection.properties => PropertiesTab(serverId: widget.serverId, running: status.isActive),
      ServerSection.files => FilesTab(serverId: widget.serverId, running: status.isActive),
      ServerSection.logs => LogsTab(serverId: widget.serverId),
      ServerSection.automation => AutomationTab(serverId: widget.serverId, autoRestart: details.autoRestart, autostart: details.autostart),
      ServerSection.plugins => AddonsTab(serverId: widget.serverId, running: status.isActive, kind: AddonKind.plugin),
      ServerSection.mods => AddonsTab(serverId: widget.serverId, running: status.isActive, kind: AddonKind.mod),
      ServerSection.worlds => WorldsTab(serverId: widget.serverId, running: status.isActive),
      ServerSection.backups => BackupsTab(serverId: widget.serverId, running: status.isActive),
    };
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
