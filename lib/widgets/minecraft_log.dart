// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';

/// Classic Minecraft palette, the same one used by panels like Aternos.
const minecraftColors = <Color>[
  Color(0xFF000000),
  Color(0xFF0000AA),
  Color(0xFF00AA00),
  Color(0xFF00AAAA),
  Color(0xFFAA0000),
  Color(0xFFAA00AA),
  Color(0xFFFFAA00),
  Color(0xFFAAAAAA),
  Color(0xFF555555),
  Color(0xFF5555FF),
  Color(0xFF55FF55),
  Color(0xFF55FFFF),
  Color(0xFFFF5555),
  Color(0xFFFF55FF),
  Color(0xFFFFFF55),
  Color(0xFFFFFFFF),
];

const _ansiDark = <int, int>{30: 0, 31: 4, 32: 2, 33: 6, 34: 1, 35: 5, 36: 3, 37: 7};
const _ansiBright = <int, int>{90: 8, 91: 12, 92: 10, 93: 14, 94: 9, 95: 13, 96: 11, 97: 15};
const _sectionColors = <String, int>{
  '0': 0,
  '1': 1,
  '2': 2,
  '3': 3,
  '4': 4,
  '5': 5,
  '6': 6,
  '7': 7,
  '8': 8,
  '9': 9,
  'a': 10,
  'b': 11,
  'c': 12,
  'd': 13,
  'e': 14,
  'f': 15,
};

class _Style {
  Color? color;
  var bold = false;
  var italic = false;
  var underline = false;
  var strike = false;

  void reset() {
    color = null;
    bold = false;
    italic = false;
    underline = false;
    strike = false;
  }

  TextStyle get textStyle {
    TextDecoration? decoration;
    if (underline && strike) {
      decoration = TextDecoration.combine([TextDecoration.underline, TextDecoration.lineThrough]);
    } else if (underline) {
      decoration = TextDecoration.underline;
    } else if (strike) {
      decoration = TextDecoration.lineThrough;
    }
    return TextStyle(
      color: color,
      fontWeight: bold ? FontWeight.w700 : null,
      fontStyle: italic ? FontStyle.italic : null,
      decoration: decoration,
    );
  }
}

/// Turns one server log line into spans. Understands ANSI SGR (Paper console)
/// and the legacy `§` codes.
List<TextSpan> minecraftLogSpans(String line) {
  final style = _Style();
  final spans = <TextSpan>[];
  final buffer = StringBuffer();

  void flush() {
    if (buffer.isEmpty) {
      return;
    }
    spans.add(TextSpan(text: buffer.toString(), style: style.textStyle));
    buffer.clear();
  }

  var i = 0;
  while (i < line.length) {
    final char = line[i];
    if (char == '\x1B' && i + 1 < line.length && line[i + 1] == '[') {
      final end = _csiEnd(line, i + 2);
      if (end < 0) {
        i++;
        continue;
      }
      if (line[end] == 'm') {
        flush();
        _applySgr(line.substring(i + 2, end), style);
      }
      i = end + 1;
      continue;
    }
    if ((char == '§' || char == '\u00A7') && i + 1 < line.length) {
      flush();
      _applySection(line[i + 1], style);
      i += 2;
      continue;
    }
    if (char != '\r') {
      buffer.write(char);
    }
    i++;
  }
  flush();
  return spans;
}

int _csiEnd(String line, int start) {
  for (var i = start; i < line.length; i++) {
    final code = line.codeUnitAt(i);
    if (code >= 0x40 && code <= 0x7E) {
      return i;
    }
  }
  return -1;
}

void _applySgr(String params, _Style style) {
  if (params.isEmpty) {
    style.reset();
    return;
  }
  final numbers = params.split(';').map((part) => part.isEmpty ? 0 : int.tryParse(part) ?? -1).toList();
  var i = 0;
  while (i < numbers.length) {
    final code = numbers[i];
    if (code == 0) {
      style.reset();
    } else if (code == 1) {
      style.bold = true;
    } else if (code == 22) {
      style.bold = false;
    } else if (code == 3) {
      style.italic = true;
    } else if (code == 23) {
      style.italic = false;
    } else if (code == 4) {
      style.underline = true;
    } else if (code == 24) {
      style.underline = false;
    } else if (code == 9) {
      style.strike = true;
    } else if (code == 29) {
      style.strike = false;
    } else if (code == 39) {
      style.color = null;
    } else if (_ansiDark.containsKey(code)) {
      style.color = minecraftColors[_ansiDark[code]!];
    } else if (_ansiBright.containsKey(code)) {
      style.color = minecraftColors[_ansiBright[code]!];
    } else if (code == 38 && i + 2 < numbers.length && numbers[i + 1] == 5) {
      style.color = _color256(numbers[i + 2]);
      i += 2;
    } else if (code == 38 && i + 4 < numbers.length && numbers[i + 1] == 2) {
      style.color = Color.fromARGB(255, numbers[i + 2].clamp(0, 255), numbers[i + 3].clamp(0, 255), numbers[i + 4].clamp(0, 255));
      i += 4;
    }
    i++;
  }
}

Color _color256(int index) {
  if (index < 16) {
    return minecraftColors[index];
  }
  if (index >= 232) {
    final level = (index - 232) * 10 + 8;
    return Color.fromARGB(255, level, level, level);
  }
  final cube = index - 16;
  final r = cube ~/ 36;
  final g = (cube ~/ 6) % 6;
  final b = cube % 6;
  int channel(int value) => value == 0 ? 0 : 55 + value * 40;
  return Color.fromARGB(255, channel(r), channel(g), channel(b));
}

void _applySection(String code, _Style style) {
  final lower = code.toLowerCase();
  final color = _sectionColors[lower];
  if (color != null) {
    style.color = minecraftColors[color];
    style.bold = false;
    style.italic = false;
    style.underline = false;
    style.strike = false;
    return;
  }
  switch (lower) {
    case 'l':
      style.bold = true;
    case 'o':
      style.italic = true;
    case 'n':
      style.underline = true;
    case 'm':
      style.strike = true;
    case 'r':
      style.reset();
  }
}

class MinecraftLogLine extends StatelessWidget {
  const MinecraftLogLine(this.line, {super.key, this.fontSize = 13, this.wrap = true, this.prefix});

  final String line;
  final double fontSize;
  final bool wrap;

  /// Shown before the line in a muted color (e.g. a timestamp).
  final String? prefix;

  @override
  Widget build(BuildContext context) {
    return Text.rich(
      TextSpan(
        children: [
          if (prefix != null) TextSpan(text: '$prefix ', style: const TextStyle(color: Color(0xFF888888))),
          ...minecraftLogSpans(line),
        ],
      ),
      softWrap: wrap,
      overflow: wrap ? TextOverflow.clip : TextOverflow.ellipsis,
      maxLines: wrap ? null : 1,
      style: TextStyle(fontFamily: 'monospace', fontFamilyFallback: const ['Consolas', 'Menlo', 'DejaVu Sans Mono'], fontSize: fontSize, height: 1.3),
    );
  }
}
