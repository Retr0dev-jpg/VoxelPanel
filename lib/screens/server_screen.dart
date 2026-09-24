// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';

class ServerScreen extends StatefulWidget {
  const ServerScreen({super.key, required this.serverId});

  final String serverId;

  @override
  State<ServerScreen> createState() => _ServerScreenState();
}

class _ServerScreenState extends State<ServerScreen> with SingleTickerProviderStateMixin {
  late final TabController _tabs = TabController(length: 6, vsync: this);
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
    _tabs.dispose();
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
      appBar: AppBar(
        title: Text(details?.name ?? 'Server'),
        bottom: TabBar(
          controller: _tabs,
          isScrollable: true,
          tabs: const [
            Tab(text: 'Panoramica'),
            Tab(text: 'Console'),
            Tab(text: 'Proprietà'),
            Tab(text: 'Plugin'),
            Tab(text: 'Mondi'),
            Tab(text: 'Backup'),
          ],
        ),
      ),
      body: details == null
          ? Center(child: _error.isEmpty ? const CircularProgressIndicator() : Text(_error))
          : TabBarView(
              controller: _tabs,
              children: [
                OverviewTab(details: details, stats: _stats, onAction: _act, onReload: _reload),
                ConsoleTab(serverId: widget.serverId, running: details.status == ServerStatus.running),
                PropertiesTab(serverId: widget.serverId),
                PluginsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
                WorldsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
                BackupsTab(serverId: widget.serverId, running: details.status != ServerStatus.stopped),
              ],
            ),
    );
  }
}

class OverviewTab extends StatelessWidget {
  const OverviewTab({
    super.key,
    required this.details,
    required this.stats,
    required this.onAction,
    required this.onReload,
  });

  final ServerDetails details;
  final ProcessStats? stats;
  final Future<bool> Function(Future<void> Function() action) onAction;
  final Future<void> Function() onReload;

  @override
  Widget build(BuildContext context) {
    final running = details.status == ServerStatus.running || details.status == ServerStatus.starting;
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(statusLabel(details.status), style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        Text('Paper ${details.paperVersion ?? 'n/d'} · Java ${details.javaMajor ?? 'n/d'}'),
        Text('RAM ${details.ramMin} / ${details.ramMax}'),
        Text(details.root),
        if (stats != null) ...[
          const SizedBox(height: 8),
          Text('PID ${stats!.pid} · CPU ${stats!.cpuPercent.toStringAsFixed(1)}% · RAM ${formatBytes(stats!.memoryBytes)}'),
        ],
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          children: [
            FilledButton(
              onPressed: running ? null : () => onAction(() => startServer(id: details.id)),
              child: const Text('Avvia'),
            ),
            FilledButton.tonal(
              onPressed: running ? () => onAction(() => stopServer(id: details.id)) : null,
              child: const Text('Stop'),
            ),
            OutlinedButton(
              onPressed: () => onAction(() => restartServer(id: details.id)),
              child: const Text('Riavvia'),
            ),
            OutlinedButton(
              onPressed: () => onAction(() => openInExplorer(path: details.root)),
              child: const Text('Apri cartella'),
            ),
          ],
        ),
        if (!details.eulaAccepted)
          Padding(
            padding: const EdgeInsets.only(top: 12),
            child: FilledButton(
              onPressed: () => onAction(() => acceptServerEula(id: details.id)),
              child: const Text("Accetta l'EULA"),
            ),
          ),
        const SizedBox(height: 24),
        RuntimeEditor(details: details, onAction: onAction, onReload: onReload),
        const SizedBox(height: 24),
        OutlinedButton(
          onPressed: () => _delete(context),
          child: const Text('Elimina server'),
        ),
      ],
    );
  }

  Future<void> _delete(BuildContext context) async {
    var deleteFiles = false;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          title: const Text('Eliminare il server?'),
          content: CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: deleteFiles,
            onChanged: (value) => setState(() => deleteFiles = value ?? false),
            title: const Text('Elimina anche i file sul disco'),
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Annulla')),
            FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Elimina')),
          ],
        ),
      ),
    );
    if (confirmed == true && context.mounted) {
      final removed = await onAction(() => deleteServer(id: details.id, deleteFiles: deleteFiles));
      if (removed && context.mounted) {
        Navigator.pop(context);
      }
    }
  }
}

class RuntimeEditor extends StatefulWidget {
  const RuntimeEditor({super.key, required this.details, required this.onAction, required this.onReload});

  final ServerDetails details;
  final Future<bool> Function(Future<void> Function() action) onAction;
  final Future<void> Function() onReload;

  @override
  State<RuntimeEditor> createState() => _RuntimeEditorState();
}

class _RuntimeEditorState extends State<RuntimeEditor> {
  List<JavaRuntimeInfo> _runtimes = [];
  List<RamChoice> _ram = [];
  List<JvmFlagChoice> _flags = [];
  var _java = '';
  var _min = '';
  var _max = '';
  final _selected = <String>{};
  var _ready = false;

  @override
  void initState() {
    super.initState();
    _java = widget.details.javaHome;
    _min = widget.details.ramMin;
    _max = widget.details.ramMax;
    _selected.addAll(widget.details.jvmFlags);
    _load();
  }

  Future<void> _load() async {
    final runtimes = await listRuntimes();
    if (!mounted) {
      return;
    }
    setState(() {
      _runtimes = runtimes;
      _ram = ramPresets();
      _flags = jvmFlagChoices();
      _ready = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const LinearProgressIndicator();
    }
    final homes = <String>{
      for (final runtime in _runtimes) runtime.path,
      if (_java.isNotEmpty) _java,
    };
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Runtime e memoria', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        DropdownButtonFormField<String>(
          initialValue: homes.contains(_java) ? _java : null,
          decoration: const InputDecoration(labelText: 'Java', border: OutlineInputBorder()),
          items: [for (final path in homes) DropdownMenuItem(value: path, child: Text(path))],
          onChanged: (value) => setState(() => _java = value ?? _java),
        ),
        const SizedBox(height: 12),
        DropdownButtonFormField<String>(
          initialValue: _ram.any((choice) => choice.value == _min) ? _min : null,
          decoration: const InputDecoration(labelText: 'RAM minima', border: OutlineInputBorder()),
          items: [for (final choice in _ram) DropdownMenuItem(value: choice.value, child: Text(choice.label))],
          onChanged: (value) => setState(() => _min = value ?? _min),
        ),
        const SizedBox(height: 12),
        DropdownButtonFormField<String>(
          initialValue: _ram.any((choice) => choice.value == _max) ? _max : null,
          decoration: const InputDecoration(labelText: 'RAM massima', border: OutlineInputBorder()),
          items: [for (final choice in _ram) DropdownMenuItem(value: choice.value, child: Text(choice.label))],
          onChanged: (value) => setState(() => _max = value ?? _max),
        ),
        for (final flag in _flags)
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: _selected.contains(flag.flag),
            title: Text(flag.flag),
            onChanged: (checked) => setState(() {
              if (checked ?? false) {
                _selected.add(flag.flag);
              } else {
                _selected.remove(flag.flag);
              }
            }),
          ),
        FilledButton(
          onPressed: () => widget.onAction(() async {
            await updateRuntimeConfig(
              id: widget.details.id,
              javaHome: _java,
              ramMin: _min,
              ramMax: _max,
              jvmFlags: _selected.toList(),
            );
            await widget.onReload();
          }),
          child: const Text('Salva runtime'),
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
              child: Text(_lines[index], style: const TextStyle(fontFamily: 'Consolas', fontSize: 13)),
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

class PropertiesTab extends StatefulWidget {
  const PropertiesTab({super.key, required this.serverId});

  final String serverId;

  @override
  State<PropertiesTab> createState() => _PropertiesTabState();
}

class _PropertiesTabState extends State<PropertiesTab> {
  final _motd = TextEditingController();
  final _port = TextEditingController();
  final _players = TextEditingController();
  final _view = TextEditingController();
  final _simulation = TextEditingController();
  final _spawn = TextEditingController();
  final _level = TextEditingController();
  final _seed = TextEditingController();
  var _online = true;
  var _whitelist = false;
  var _pvp = true;
  var _difficulty = 'easy';
  var _gamemode = 'survival';
  var _ready = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    for (final controller in [_motd, _port, _players, _view, _simulation, _spawn, _level, _seed]) {
      controller.dispose();
    }
    super.dispose();
  }

  Future<void> _load() async {
    final settings = await getSettings(id: widget.serverId);
    if (!mounted) {
      return;
    }
    _motd.text = settings.motd;
    _port.text = settings.port.toString();
    _players.text = settings.maxPlayers.toString();
    _view.text = settings.viewDistance.toString();
    _simulation.text = settings.simulationDistance.toString();
    _spawn.text = settings.spawnProtection.toString();
    _level.text = settings.levelName;
    _seed.text = settings.levelSeed;
    setState(() {
      _online = settings.onlineMode;
      _whitelist = settings.whiteList;
      _pvp = settings.pvp;
      _difficulty = settings.difficulty;
      _gamemode = settings.gamemode;
      _ready = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        TextField(controller: _motd, decoration: const InputDecoration(labelText: 'MOTD')),
        TextField(controller: _port, decoration: const InputDecoration(labelText: 'Porta')),
        TextField(controller: _players, decoration: const InputDecoration(labelText: 'Giocatori massimi')),
        DropdownButtonFormField<String>(
          initialValue: _difficulty,
          decoration: const InputDecoration(labelText: 'Difficoltà'),
          items: const [
            DropdownMenuItem(value: 'peaceful', child: Text('Pacifica')),
            DropdownMenuItem(value: 'easy', child: Text('Facile')),
            DropdownMenuItem(value: 'normal', child: Text('Normale')),
            DropdownMenuItem(value: 'hard', child: Text('Difficile')),
          ],
          onChanged: (value) => setState(() => _difficulty = value ?? _difficulty),
        ),
        DropdownButtonFormField<String>(
          initialValue: _gamemode,
          decoration: const InputDecoration(labelText: 'Gamemode'),
          items: const [
            DropdownMenuItem(value: 'survival', child: Text('Survival')),
            DropdownMenuItem(value: 'creative', child: Text('Creative')),
            DropdownMenuItem(value: 'adventure', child: Text('Adventure')),
            DropdownMenuItem(value: 'spectator', child: Text('Spectator')),
          ],
          onChanged: (value) => setState(() => _gamemode = value ?? _gamemode),
        ),
        TextField(controller: _view, decoration: const InputDecoration(labelText: 'View distance')),
        TextField(controller: _simulation, decoration: const InputDecoration(labelText: 'Simulation distance')),
        TextField(controller: _spawn, decoration: const InputDecoration(labelText: 'Spawn protection')),
        TextField(controller: _level, decoration: const InputDecoration(labelText: 'Nome mondo')),
        TextField(controller: _seed, decoration: const InputDecoration(labelText: 'Seed')),
        SwitchListTile(value: _online, onChanged: (value) => setState(() => _online = value), title: const Text('Online mode')),
        SwitchListTile(value: _whitelist, onChanged: (value) => setState(() => _whitelist = value), title: const Text('Whitelist')),
        SwitchListTile(value: _pvp, onChanged: (value) => setState(() => _pvp = value), title: const Text('PvP')),
        const SizedBox(height: 12),
        FilledButton(onPressed: _save, child: const Text('Salva proprietà')),
      ],
    );
  }

  Future<void> _save() async {
    try {
      await saveSettings(
        id: widget.serverId,
        settings: ServerSettings(
          motd: _motd.text,
          port: int.parse(_port.text),
          maxPlayers: int.parse(_players.text),
          onlineMode: _online,
          difficulty: _difficulty,
          gamemode: _gamemode,
          viewDistance: int.parse(_view.text),
          simulationDistance: int.parse(_simulation.text),
          whiteList: _whitelist,
          pvp: _pvp,
          spawnProtection: int.parse(_spawn.text),
          levelName: _level.text,
          levelSeed: _seed.text,
        ),
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
