// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:async';
import 'dart:collection';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';
import 'package:voxel_panel/widgets/minecraft_log.dart';

const consoleMaxLines = 2000;
const _historyLimit = 100;

enum LogLevel { info, warn, error }

final _warnPattern = RegExp(r'\bWARN(ING)?\b');
final _errorPattern = RegExp(r'\b(ERROR|SEVERE|FATAL)\b|Exception');

LogLevel levelOf(String line) {
  if (_errorPattern.hasMatch(line)) {
    return LogLevel.error;
  }
  if (_warnPattern.hasMatch(line)) {
    return LogLevel.warn;
  }
  return LogLevel.info;
}

/// Fixed-size buffer: the oldest lines are dropped once [capacity] is reached.
class LineBuffer {
  LineBuffer(this.capacity);

  final int capacity;
  final _lines = ListQueue<String>();

  int get length => _lines.length;
  Iterable<String> get lines => _lines;

  void add(String line) {
    if (_lines.length >= capacity) {
      _lines.removeFirst();
    }
    _lines.addLast(line);
  }

  void clear() => _lines.clear();
}

/// Up/down navigation over previously sent commands, like a shell.
class CommandHistory {
  final _entries = <String>[];
  var _cursor = 0;

  void push(String command) {
    if (_entries.isEmpty || _entries.last != command) {
      _entries.add(command);
      if (_entries.length > _historyLimit) {
        _entries.removeAt(0);
      }
    }
    _cursor = _entries.length;
  }

  String? previous() {
    if (_entries.isEmpty) {
      return null;
    }
    _cursor = (_cursor - 1).clamp(0, _entries.length - 1);
    return _entries[_cursor];
  }

  String? next() {
    if (_entries.isEmpty) {
      return null;
    }
    _cursor = (_cursor + 1).clamp(0, _entries.length);
    return _cursor == _entries.length ? '' : _entries[_cursor];
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
  final _buffer = LineBuffer(consoleMaxLines);
  final _history = CommandHistory();
  final _input = TextEditingController();
  final _search = TextEditingController();
  final _scroll = ScrollController();
  final _inputFocus = FocusNode();
  StreamSubscription<String>? _subscription;
  final _levels = <LogLevel>{...LogLevel.values};
  var _followTail = true;
  var _scheduled = false;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_trackTail);
    _subscription = watchConsole(id: widget.serverId).listen(_onLine);
  }

  @override
  void dispose() {
    _subscription?.cancel();
    _input.dispose();
    _search.dispose();
    _scroll.dispose();
    _inputFocus.dispose();
    super.dispose();
  }

  void _trackTail() {
    if (!_scroll.hasClients) {
      return;
    }
    final position = _scroll.position;
    _followTail = position.pixels >= position.maxScrollExtent - 24;
  }

  /// Batches rebuilds to one per frame; a busy server can print hundreds of lines per second.
  void _onLine(String line) {
    _buffer.add(line);
    if (_scheduled || !mounted) {
      return;
    }
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted) {
        return;
      }
      setState(() {});
      if (_followTail) {
        WidgetsBinding.instance.addPostFrameCallback((_) => _jumpToEnd());
      }
    });
    WidgetsBinding.instance.scheduleFrame();
  }

  void _jumpToEnd() {
    if (_scroll.hasClients) {
      _scroll.jumpTo(_scroll.position.maxScrollExtent);
    }
  }

  List<String> get _visible {
    final query = _search.text.trim().toLowerCase();
    return _buffer.lines.where((line) {
      if (!_levels.contains(levelOf(line))) {
        return false;
      }
      return query.isEmpty || line.toLowerCase().contains(query);
    }).toList();
  }

  Future<void> _send(String command) async {
    final text = command.trim();
    if (text.isEmpty) {
      return;
    }
    _history.push(text);
    _input.clear();
    _followTail = true;
    await runGuarded(context, () => sendCommand(id: widget.serverId, command: text));
    _inputFocus.requestFocus();
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }
    final value = switch (event.logicalKey) {
      LogicalKeyboardKey.arrowUp => _history.previous(),
      LogicalKeyboardKey.arrowDown => _history.next(),
      _ => null,
    };
    if (value == null) {
      return KeyEventResult.ignored;
    }
    _input.value = TextEditingValue(text: value, selection: TextSelection.collapsed(offset: value.length));
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final visible = _visible;
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 12, 28, 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SizedBox(
                width: 260,
                child: TextField(
                  controller: _search,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(prefixIcon: const Icon(Icons.search), hintText: l.consoleSearch, isDense: true),
                ),
              ),
              for (final level in LogLevel.values)
                FilterChip(
                  label: Text(switch (level) {
                    LogLevel.info => l.logInfo,
                    LogLevel.warn => l.logWarn,
                    LogLevel.error => l.logError,
                  }),
                  selected: _levels.contains(level),
                  onSelected: (selected) => setState(() => selected ? _levels.add(level) : _levels.remove(level)),
                ),
              IconButton(
                tooltip: l.consoleCopy,
                onPressed: () async {
                  await Clipboard.setData(ClipboardData(text: visible.join('\n')));
                  if (context.mounted) {
                    showMessage(context, l.copied);
                  }
                },
                icon: const Icon(Icons.copy_all_outlined),
              ),
              IconButton(tooltip: l.consoleClear, onPressed: () => setState(_buffer.clear), icon: const Icon(Icons.clear_all)),
              IconButton(
                tooltip: l.consoleScrollEnd,
                onPressed: () {
                  _followTail = true;
                  _jumpToEnd();
                },
                icon: const Icon(Icons.vertical_align_bottom),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Expanded(
            child: Container(
              decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12), border: Border.all(color: colors.cardBorder)),
              child: visible.isEmpty
                  ? Center(child: Text(widget.running ? l.consoleWaiting : l.consoleEmpty, style: TextStyle(color: colors.muted)))
                  : SelectionArea(
                      child: ListView.builder(
                        controller: _scroll,
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        itemCount: visible.length,
                        itemBuilder: (context, index) => Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 1),
                          child: DefaultTextStyle.merge(style: const TextStyle(color: Color(0xFFDDDDDD)), child: MinecraftLogLine(visible[index])),
                        ),
                      ),
                    ),
            ),
          ),
          const SizedBox(height: 12),
          Focus(
            onKeyEvent: _onKey,
            child: TextField(
              controller: _input,
              focusNode: _inputFocus,
              enabled: widget.running,
              style: const TextStyle(fontFamily: 'monospace'),
              decoration: InputDecoration(
                prefixIcon: const Icon(Icons.chevron_right),
                hintText: widget.running ? l.consoleCommandHint : l.consoleNotRunning,
                suffixIcon: IconButton(onPressed: widget.running ? () => _send(_input.text) : null, icon: const Icon(Icons.send)),
              ),
              onSubmitted: _send,
            ),
          ),
        ],
      ),
    );
  }
}
