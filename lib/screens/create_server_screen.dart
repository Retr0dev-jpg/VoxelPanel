import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:voxel_panel/src/labels.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/widgets/create_wizard_body.dart';

class CreateServerScreen extends StatefulWidget {
  const CreateServerScreen({super.key});

  @override
  State<CreateServerScreen> createState() => _CreateServerScreenState();
}

class _CreateServerScreenState extends State<CreateServerScreen> {
  List<String> _versions = [];
  List<RamChoice> _ram = [];
  List<JvmFlagChoice> _flags = [];
  List<JavaRuntimeInfo> _runtimes = [];
  List<JavaReleaseInfo> _releases = [];
  var _min = '2G';
  var _max = '4G';
  var _ready = false;
  var _busy = false;
  var _error = '';
  final _progress = <String>[];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final versions = await listPaperVersions();
      final releases = await listJavaReleases();
      final runtimes = await listRuntimes();
      final suggestion = suggestRam();
      if (!mounted) {
        return;
      }
      setState(() {
        _versions = versions;
        _releases = releases;
        _runtimes = runtimes;
        _ram = ramPresets();
        _flags = jvmFlagChoices();
        _min = suggestion.ramMin;
        _max = suggestion.ramMax;
        _ready = true;
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = readableError(error));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Nuovo server')),
      body: !_ready
          ? Center(child: _error.isEmpty ? const CircularProgressIndicator() : Text(_error))
          : CreateWizardBody(
              paperVersions: _versions,
              ramChoices: _ram,
              jvmFlags: _flags,
              runtimes: _runtimes,
              javaReleases: _releases,
              suggestedMin: _min,
              suggestedMax: _max,
              progress: _progress,
              busy: _busy,
              onAuto: (request) => _install(installAuto(request: request)),
              onManual: (request) => _install(installManual(request: request)),
              pickDirectory: () => FilePicker.getDirectoryPath(dialogTitle: 'Cartella del server'),
              pickJar: _pickJar,
              installJava: _installJava,
            ),
    );
  }

  Future<String?> _pickJar() async {
    final files = await FilePicker.pickFiles(
      dialogTitle: 'Jar Paper',
      type: FileType.custom,
      allowedExtensions: const ['jar'],
    );
    if (files.isEmpty) {
      return null;
    }
    return files.first.path;
  }

  Future<void> _installJava(int major) async {
    setState(() {
      _busy = true;
      _progress.clear();
    });
    try {
      await _collect(installJava(major: major));
      final runtimes = await listRuntimes();
      if (mounted) {
        setState(() => _runtimes = runtimes);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(readableError(error))));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _install(Stream<ProgressEvent> stream) async {
    setState(() {
      _busy = true;
      _progress.clear();
    });
    try {
      final id = await _collect(stream);
      if (mounted && id != null) {
        Navigator.pop(context, true);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _progress.add(readableError(error)));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<String?> _collect(Stream<ProgressEvent> stream) async {
    String? serverId;
    await for (final event in stream) {
      if (!mounted) {
        break;
      }
      setState(() => _progress.add('${event.stage}: ${event.message}'));
      if (event.error != null) {
        throw Exception(event.error);
      }
      serverId = event.serverId ?? serverId;
    }
    return serverId;
  }
}
