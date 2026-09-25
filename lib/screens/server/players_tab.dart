// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/players.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/common/panel_card.dart';

class PlayersTab extends StatefulWidget {
  const PlayersTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<PlayersTab> createState() => _PlayersTabState();
}

class _PlayersTabState extends State<PlayersTab> {
  List<String> _online = [];
  PlayerLists? _lists;
  Object? _error;
  var _loading = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(PlayersTab oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.running != widget.running) {
      _load();
    }
  }

  Future<void> _load() async {
    setState(() => _loading = true);
    try {
      final lists = await playerLists(id: widget.serverId);
      final online = await onlinePlayers(id: widget.serverId);
      if (mounted) {
        setState(() {
          _lists = lists;
          _online = online;
          _error = null;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _command(String command) async {
    final l = context.l10n;
    try {
      final response = await runServerCommand(id: widget.serverId, command: command);
      if (mounted) {
        showMessage(context, response.trim().isEmpty ? l.commandSent : response.trim());
      }
      await Future<void>.delayed(const Duration(milliseconds: 400));
      await _load();
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    }
  }

  Future<void> _modify(PlayerListKind list, bool add, String target, {String reason = ''}) async {
    if (await runGuarded(context, () => modifyPlayerList(id: widget.serverId, list: list, add: add, target: target, reason: reason))) {
      await Future<void>.delayed(const Duration(milliseconds: 300));
      await _load();
    }
  }

  Future<void> _toggleWhitelist(bool enabled) async {
    if (widget.running) {
      await _command(enabled ? 'whitelist on' : 'whitelist off');
      return;
    }
    if (await runGuarded(context, () => saveProperties(id: widget.serverId, entries: [PropertyEntry(key: 'white-list', value: '$enabled')]))) {
      await _load();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final lists = _lists;
    if (_error != null) {
      return ErrorState(error: _error!, onRetry: _load);
    }
    if (lists == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 12, 28, 28),
      children: [
        if (_loading) const LinearProgressIndicator(),
        Text(widget.running ? l.playersLiveHint : l.playersOfflineHint, style: TextStyle(color: context.voxel.muted)),
        const SizedBox(height: 12),
        PanelCard(
          title: l.playersOnline(_online.length),
          icon: Icons.people_outline,
          trailing: IconButton(tooltip: l.refresh, onPressed: _load, icon: const Icon(Icons.refresh)),
          child: !widget.running
              ? Text(l.serverOffline)
              : _online.isEmpty
              ? Text(l.noPlayersOnline)
              : Column(children: [for (final name in _online) _onlineTile(context, name)]),
        ),
        const SizedBox(height: 12),
        _ListCard(
          title: l.whitelist,
          icon: Icons.verified_user_outlined,
          hint: l.playerNameHint,
          entries: [for (final player in lists.whitelist) (player.name, player.uuid)],
          onAdd: (name) => _modify(PlayerListKind.whitelist, true, name),
          onRemove: (name) => _modify(PlayerListKind.whitelist, false, name),
          header: SwitchListTile(contentPadding: EdgeInsets.zero, title: Text(l.whitelistEnabled), value: lists.whitelistEnabled, onChanged: _toggleWhitelist),
        ),
        const SizedBox(height: 12),
        _ListCard(
          title: l.operators,
          icon: Icons.shield_outlined,
          hint: l.playerNameHint,
          entries: [for (final player in lists.operators) (player.name, l.opLevel(player.detail))],
          onAdd: (name) => _modify(PlayerListKind.operators, true, name),
          onRemove: (name) => _modify(PlayerListKind.operators, false, name),
        ),
        const SizedBox(height: 12),
        _ListCard(
          title: l.bannedPlayers,
          icon: Icons.block,
          hint: l.playerNameHint,
          entries: [for (final player in lists.bannedPlayers) (player.name, player.detail)],
          onAdd: (name) => _modify(PlayerListKind.bannedPlayers, true, name),
          onRemove: (name) => _modify(PlayerListKind.bannedPlayers, false, name),
        ),
        const SizedBox(height: 12),
        _ListCard(
          title: l.bannedIps,
          icon: Icons.public_off,
          hint: l.ipHint,
          entries: [for (final ban in lists.bannedIps) (ban.ip, ban.reason)],
          onAdd: (ip) => _modify(PlayerListKind.bannedIps, true, ip),
          onRemove: (ip) => _modify(PlayerListKind.bannedIps, false, ip),
        ),
      ],
    );
  }

  Widget _onlineTile(BuildContext context, String name) {
    final l = context.l10n;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: CircleAvatar(child: Text(name.characters.first.toUpperCase())),
      title: Text(name),
      trailing: PopupMenuButton<String>(
        tooltip: l.moreActions,
        onSelected: (action) async {
          switch (action) {
            case 'kick':
              await _command('kick $name');
            case 'ban':
              await _modify(PlayerListKind.bannedPlayers, true, name);
            case 'op':
              await _modify(PlayerListKind.operators, true, name);
            case 'deop':
              await _modify(PlayerListKind.operators, false, name);
            default:
              await _command('gamemode $action $name');
          }
        },
        itemBuilder: (context) => [
          PopupMenuItem(value: 'kick', child: Text(l.kick)),
          PopupMenuItem(value: 'ban', child: Text(l.ban)),
          PopupMenuItem(value: 'op', child: Text(l.makeOp)),
          PopupMenuItem(value: 'deop', child: Text(l.removeOp)),
          const PopupMenuDivider(),
          PopupMenuItem(value: 'survival', child: Text(l.gamemodeSurvival)),
          PopupMenuItem(value: 'creative', child: Text(l.gamemodeCreative)),
          PopupMenuItem(value: 'adventure', child: Text(l.gamemodeAdventure)),
          PopupMenuItem(value: 'spectator', child: Text(l.gamemodeSpectator)),
        ],
      ),
    );
  }
}

class _ListCard extends StatefulWidget {
  const _ListCard({required this.title, required this.icon, required this.hint, required this.entries, required this.onAdd, required this.onRemove, this.header});

  final String title;
  final IconData icon;
  final String hint;
  final List<(String, String)> entries;
  final Future<void> Function(String value) onAdd;
  final Future<void> Function(String value) onRemove;
  final Widget? header;

  @override
  State<_ListCard> createState() => _ListCardState();
}

class _ListCardState extends State<_ListCard> {
  final _input = TextEditingController();

  @override
  void dispose() {
    _input.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final value = _input.text.trim();
    if (value.isEmpty) {
      return;
    }
    await widget.onAdd(value);
    _input.clear();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return PanelCard(
      title: '${widget.title} (${widget.entries.length})',
      icon: widget.icon,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ?widget.header,
          Row(
            children: [
              Expanded(child: TextField(controller: _input, decoration: InputDecoration(hintText: widget.hint, isDense: true), onSubmitted: (_) => _submit())),
              const SizedBox(width: 8),
              FilledButton.tonal(onPressed: _submit, child: Text(l.add)),
            ],
          ),
          for (final (name, detail) in widget.entries)
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              title: Text(name),
              subtitle: detail.isEmpty ? null : Text(detail, maxLines: 1, overflow: TextOverflow.ellipsis),
              trailing: IconButton(tooltip: l.remove, onPressed: () => widget.onRemove(name), icon: const Icon(Icons.close)),
            ),
        ],
      ),
    );
  }
}
