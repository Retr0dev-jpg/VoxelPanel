// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/widgets/app_sidebar.dart';
import 'package:voxel_panel/widgets/server_dashboard.dart';
import 'package:voxel_panel/widgets/minecraft_log.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

class ServerScreen extends StatefulWidget {
  const ServerScreen({super.key, required this.serverId});

  final String serverId;

  @override
  State<ServerScreen> createState() => _ServerScreenState();
}

class _ServerScreenState extends State<ServerScreen> {
  var _section = 0;
  ServerDetails? _details;
  ProcessStats? _stats;
  Timer? _timer;
  var _error = '';

  @override
  void initState() {
    super.initState();
    _reload();
    _timer = Timer.periodic(const Duration(seconds: 2), (_) => _reload(quiet: true));
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _reload({bool quiet = false}) async {
    try {
      final details = await getServer(id: widget.serverId);
      final stats = await serverStats(id: widget.serverId);
      if (!mounted) {
        return;
      }
      setState(() {
        _details = details;
        _stats = stats;
        _error = '';
      });
    } catch (error) {
      if (mounted && !quiet) {
        setState(() => _error = readableError(error));
      }
    }
  }

  Future<bool> _act(Future<void> Function() action) async {
    try {
      await action();
      await _reload();
      return true;
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
      return false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final details = _details;
    return Scaffold(
      body: Row(
        children: [
          AppSidebar(
            onBack: () => Navigator.pop(context),
            selected: _section,
            onSelected: (index) => setState(() => _section = index),
            entries: const [
              SidebarEntry(label: 'Panoramica', icon: Icons.space_dashboard_outlined),
              SidebarEntry(label: 'Console', icon: Icons.terminal_outlined),
              SidebarEntry(label: 'Proprietà', icon: Icons.tune),
              SidebarEntry(label: 'Plugin', icon: Icons.extension_outlined),
              SidebarEntry(label: 'Mondi', icon: Icons.public_outlined),
              SidebarEntry(label: 'Backup', icon: Icons.inventory_2_outlined),
            ],
          ),
          Expanded(
            child: details == null
                ? Center(child: _error.isEmpty ? const CircularProgressIndicator() : Text(_error))
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (_section != 0)
                        Padding(
                          padding: const EdgeInsets.fromLTRB(28, 22, 28, 8),
                          child: Text(details.name, style: const TextStyle(fontSize: 28, fontWeight: FontWeight.w700)),
                        ),
                      Expanded(
                        child: IndexedStack(
                          index: _section,
                          children: [
                            OverviewTab(details: details, stats: _stats, onAction: _act),
                            ConsoleTab(serverId: widget.serverId, running: details.status == ServerStatus.running),
                            PropertiesTab(serverId: widget.serverId),
                            PluginsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
                            WorldsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
                            BackupsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
                          ],
                        ),
                      ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }
}

class OverviewTab extends StatelessWidget {
  const OverviewTab({super.key, required this.details, required this.stats, required this.onAction});

  final ServerDetails details;
  final ProcessStats? stats;
  final Future<bool> Function(Future<void> Function() action) onAction;

  @override
  Widget build(BuildContext context) {
    return ListView(
      children: [
        ServerDashboard(details: details, stats: stats, onAction: onAction),
        if (!details.eulaAccepted)
          Padding(
            padding: const EdgeInsets.fromLTRB(28, 0, 28, 28),
            child: FilledButton(
              onPressed: () => onAction(() => acceptServerEula(id: details.id)),
              child: const Text("Accetta l'EULA"),
            ),
          ),
      ],
    );
  }
}

class ConsoleTab extends StatefulWidget {
  const ConsoleTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<ConsoleTab> createState() => _ConsoleTabState();
}

class _ConsoleTabState extends State<ConsoleTab> {
  final _lines = <String>[];
  final _input = TextEditingController();
  final _scroll = ScrollController();
  StreamSubscription<String>? _subscription;

  @override
  void initState() {
    super.initState();
    _subscription = watchConsole(id: widget.serverId).listen((line) {
      if (!mounted) {
        return;
      }
      setState(() => _lines.add(line));
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scroll.hasClients) {
          _scroll.jumpTo(_scroll.position.maxScrollExtent);
        }
      });
    });
  }

  @override
  void dispose() {
    _subscription?.cancel();
    _input.dispose();
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: ListView.builder(
            controller: _scroll,
            itemCount: _lines.length,
            itemBuilder: (context, index) => Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
              child: MinecraftLogLine(_lines[index]),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _input,
            enabled: widget.running,
            decoration: const InputDecoration(
              hintText: 'Comando',
              border: OutlineInputBorder(),
            ),
            onSubmitted: (command) async {
              _input.clear();
              try {
                await sendCommand(id: widget.serverId, command: command);
              } catch (error) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
                }
              }
            },
          ),
        ),
      ],
    );
  }
}

const _propertyLabels = {
  'motd': 'MOTD',
  'server-port': 'Porta',
  'max-players': 'Giocatori massimi',
  'difficulty': 'Difficoltà',
  'gamemode': 'Gamemode',
  'view-distance': 'View distance',
  'simulation-distance': 'Simulation distance',
  'spawn-protection': 'Spawn protection',
  'level-name': 'Nome mondo',
  'level-seed': 'Seed',
  'online-mode': 'Online mode',
  'white-list': 'Whitelist',
  'pvp': 'PvP',
};

const _difficultyOptions = {
  'peaceful': 'Pacifica',
  'easy': 'Facile',
  'normal': 'Normale',
  'hard': 'Difficile',
};

const _gamemodeOptions = {
  'survival': 'Survival',
  'creative': 'Creative',
  'adventure': 'Adventure',
  'spectator': 'Spectator',
};

enum _PropertyKind { text, toggle, difficulty, gamemode }

class _PropertyField {
  _PropertyField(this.key, String value)
    : kind = _kindFor(key, value),
      controller = _kindFor(key, value) == _PropertyKind.toggle ? null : TextEditingController(text: value),
      enabled = value.toLowerCase() == 'true';

  final String key;
  final _PropertyKind kind;
  final TextEditingController? controller;
  bool enabled;

  static _PropertyKind _kindFor(String key, String value) {
    if (key == 'difficulty' && _difficultyOptions.containsKey(value)) {
      return _PropertyKind.difficulty;
    }
    if (key == 'gamemode' && _gamemodeOptions.containsKey(value)) {
      return _PropertyKind.gamemode;
    }
    if (value == 'true' || value == 'false') {
      return _PropertyKind.toggle;
    }
    return _PropertyKind.text;
  }

  String get value {
    if (kind == _PropertyKind.toggle) {
      return enabled ? 'true' : 'false';
    }
    return controller?.text ?? '';
  }

  String get label => _propertyLabels[key] ?? key;

  void dispose() => controller?.dispose();
}

class PropertiesTab extends StatefulWidget {
  const PropertiesTab({super.key, required this.serverId});

  final String serverId;

  @override
  State<PropertiesTab> createState() => _PropertiesTabState();
}

class _PropertiesTabState extends State<PropertiesTab> {
  final _fields = <_PropertyField>[];
  String? _error;
  var _ready = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final field in _fields) {
      field.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final entries = await listProperties(id: widget.serverId);
      if (!mounted) {
        return;
      }
      setState(() {
        _fields
          ..clear()
          ..addAll(entries.map((entry) => _PropertyField(entry.key, entry.value)));
        _error = null;
        _ready = true;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = readableError(error);
        _ready = true;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(child: Text(_error!));
    }
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        for (final field in _fields) _editor(field),
        const SizedBox(height: 12),
        FilledButton(onPressed: _save, child: const Text('Salva proprietà')),
      ],
    );
  }

  Widget _editor(_PropertyField field) {
    switch (field.kind) {
      case _PropertyKind.toggle:
        return SwitchListTile(
          value: field.enabled,
          title: Text(field.label),
          subtitle: field.label == field.key ? null : Text(field.key),
          onChanged: (value) => setState(() => field.enabled = value),
        );
      case _PropertyKind.difficulty:
        return _choice(field, _difficultyOptions);
      case _PropertyKind.gamemode:
        return _choice(field, _gamemodeOptions);
      case _PropertyKind.text:
        return TextField(
          controller: field.controller,
          decoration: InputDecoration(labelText: field.label, helperText: field.label == field.key ? null : field.key),
        );
    }
  }

  Widget _choice(_PropertyField field, Map<String, String> options) {
    return DropdownButtonFormField<String>(
      initialValue: field.controller?.text,
      decoration: InputDecoration(labelText: field.label),
      items: [
        for (final option in options.entries) DropdownMenuItem(value: option.key, child: Text(option.value)),
      ],
      onChanged: (value) {
        if (value != null) {
          field.controller?.text = value;
        }
      },
    );
  }

  Future<void> _save() async {
    try {
      await saveProperties(
        id: widget.serverId,
        entries: [for (final field in _fields) PropertyEntry(key: field.key, value: field.value)],
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Proprietà salvate. Riavvia il server per applicarle.')));
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }
}

class PluginsTab extends StatefulWidget {
  const PluginsTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<PluginsTab> createState() => _PluginsTabState();
}

class _PluginsTabState extends State<PluginsTab> {
  List<PluginInfo> _plugins = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final plugins = await listPlugins(id: widget.serverId);
    if (mounted) {
      setState(() => _plugins = plugins);
    }
  }

  Future<void> _act(Future<void> Function() action) async {
    try {
      await action();
      await _load();
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: Wrap(
            spacing: 8,
            children: [
              FilledButton(
                onPressed: widget.running
                    ? null
                    : () async {
                        final files = await FilePicker.pickFiles(
                          dialogTitle: 'Jar del plugin',
                          type: FileType.custom,
                          allowedExtensions: const ['jar'],
                        );
                        if (files.isEmpty || files.first.path == null) {
                          return;
                        }
                        await _act(() => installPluginFile(id: widget.serverId, sourcePath: files.first.path!));
                      },
                child: const Text('Installa jar'),
              ),
              FilledButton.tonal(
                onPressed: widget.running ? null : () => _search(context),
                child: const Text('Cerca su Modrinth'),
              ),
            ],
          ),
        ),
        if (widget.running)
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 12),
            child: Text('Ferma il server prima di modificare i plugin.'),
          ),
        Expanded(
          child: ListView.builder(
            itemCount: _plugins.length,
            itemBuilder: (context, index) {
              final plugin = _plugins[index];
              return ListTile(
                title: Text(plugin.fileName),
                subtitle: Text(formatBytes(plugin.sizeBytes)),
                trailing: Wrap(
                  children: [
                    Switch(
                      value: plugin.enabled,
                      onChanged: widget.running
                          ? null
                          : (value) => _act(
                              () => setPluginEnabled(id: widget.serverId, fileName: plugin.fileName, enabled: value),
                            ),
                    ),
                    IconButton(
                      onPressed: widget.running
                          ? null
                          : () => _act(() => deletePlugin(id: widget.serverId, fileName: plugin.fileName)),
                      icon: const Icon(Icons.delete),
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ],
    );
  }

  Future<void> _search(BuildContext context) async {
    final query = TextEditingController();
    List<ModrinthProject> hits = [];
    await showDialog<void>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          title: const Text('Modrinth'),
          content: SizedBox(
            width: 420,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: query,
                  decoration: const InputDecoration(labelText: 'Cerca plugin Paper'),
                  onSubmitted: (value) async {
                    final found = await searchModrinth(query: value);
                    setState(() => hits = found);
                  },
                ),
                SizedBox(
                  height: 240,
                  child: ListView(
                    children: [
                      for (final hit in hits)
                        ListTile(
                          title: Text(hit.title),
                          subtitle: Text(hit.description),
                          onTap: () async {
                            Navigator.pop(context);
                            await _installModrinth(hit.projectId);
                          },
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Chiudi'))],
        ),
      ),
    );
    query.dispose();
  }

  Future<void> _installModrinth(String projectId) async {
    try {
      await for (final event in installModrinthProject(id: widget.serverId, projectId: projectId)) {
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      await _load();
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }
}

class WorldsTab extends StatefulWidget {
  const WorldsTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<WorldsTab> createState() => _WorldsTabState();
}

class _WorldsTabState extends State<WorldsTab> {
  List<WorldInfo> _worlds = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final worlds = await listWorlds(id: widget.serverId);
    if (mounted) {
      setState(() => _worlds = worlds);
    }
  }

  Future<void> _act(Future<void> Function() action) async {
    try {
      await action();
      await _load();
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_worlds.isEmpty) {
      return const Center(child: Text('Nessun mondo. Verrà creato al primo avvio.'));
    }
    return ListView.builder(
      itemCount: _worlds.length,
      itemBuilder: (context, index) {
        final world = _worlds[index];
        return ListTile(
          title: Text(world.active ? '${world.name} (attivo)' : world.name),
          subtitle: Text(formatBytes(world.sizeBytes)),
          trailing: Wrap(
            children: [
              IconButton(
                tooltip: 'Apri cartella',
                onPressed: () => openInExplorer(path: world.path),
                icon: const Icon(Icons.folder_open),
              ),
              IconButton(
                tooltip: 'Imposta come mondo attivo',
                onPressed: world.active ? null : () => _act(() => setActiveWorld(id: widget.serverId, name: world.name)),
                icon: const Icon(Icons.check),
              ),
              IconButton(
                tooltip: 'Elimina',
                onPressed: widget.running ? null : () => _act(() => deleteWorld(id: widget.serverId, name: world.name)),
                icon: const Icon(Icons.delete),
              ),
            ],
          ),
        );
      },
    );
  }
}

class BackupsTab extends StatefulWidget {
  const BackupsTab({super.key, required this.serverId, required this.running});

  final String serverId;
  final bool running;

  @override
  State<BackupsTab> createState() => _BackupsTabState();
}

class _BackupsTabState extends State<BackupsTab> {
  List<BackupInfo> _backups = [];
  var _busy = false;
  var _message = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final backups = await listBackups(id: widget.serverId);
    if (mounted) {
      setState(() => _backups = backups);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              FilledButton(
                onPressed: _busy ? null : () => _run(createBackup(id: widget.serverId)),
                child: const Text('Crea backup'),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  widget.running
                      ? 'Un backup a server fermo è più coerente. $_message'
                      : _message,
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView.builder(
            itemCount: _backups.length,
            itemBuilder: (context, index) {
              final backup = _backups[index];
              return ListTile(
                title: Text(backup.fileName),
                subtitle: Text(formatBytes(backup.sizeBytes)),
                trailing: Wrap(
                  children: [
                    TextButton(
                      onPressed: widget.running || _busy
                          ? null
                          : () => _run(restoreBackup(id: widget.serverId, fileName: backup.fileName)),
                      child: const Text('Ripristina'),
                    ),
                    IconButton(
                      onPressed: _busy
                          ? null
                          : () async {
                              await deleteBackup(id: widget.serverId, fileName: backup.fileName);
                              await _load();
                            },
                      icon: const Icon(Icons.delete),
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ],
    );
  }

  Future<void> _run(Stream<ProgressEvent> stream) async {
    setState(() {
      _busy = true;
      _message = '';
    });
    try {
      await for (final event in stream) {
        if (!mounted) {
          return;
        }
        setState(() => _message = event.message);
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
      await _load();
    } catch (error) {
      if (mounted) {
        setState(() => _message = readableError(error));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }
}
