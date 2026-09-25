// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';
import 'package:voxel_panel/src/theme.dart';

enum CodeLanguage { yaml, toml, properties, json, plain }

CodeLanguage languageOf(String fileName) {
  final name = fileName.toLowerCase();
  if (name.endsWith('.yml') || name.endsWith('.yaml')) {
    return CodeLanguage.yaml;
  }
  if (name.endsWith('.toml') || name.endsWith('.conf') || name.endsWith('.cfg') || name.endsWith('.ini')) {
    return CodeLanguage.toml;
  }
  if (name.endsWith('.properties') || name.endsWith('.txt')) {
    return CodeLanguage.properties;
  }
  if (name.endsWith('.json') || name.endsWith('.json5') || name.endsWith('.mcmeta')) {
    return CodeLanguage.json;
  }
  return CodeLanguage.plain;
}

class _Rule {
  const _Rule(this.pattern, this.style);

  final RegExp pattern;
  final TextStyle style;
}

/// Highlights config files with a few regex rules; cheap enough to run on every keystroke.
class HighlightingController extends TextEditingController {
  HighlightingController({super.text, required this.language});

  CodeLanguage language;

  List<_Rule> _rules(VoxelColors colors) {
    final comment = TextStyle(color: colors.muted, fontStyle: FontStyle.italic);
    final key = TextStyle(color: colors.accent, fontWeight: FontWeight.w600);
    final string = TextStyle(color: colors.online);
    final number = TextStyle(color: colors.warning);
    final keyword = TextStyle(color: colors.restart);
    return switch (language) {
      CodeLanguage.yaml => [
        _Rule(RegExp(r'#.*$', multiLine: true), comment),
        _Rule(RegExp(r'''^\s*-?\s*[\w.\-"']+(?=\s*:)''', multiLine: true), key),
        _Rule(RegExp(r'"[^"\n]*"'), string),
        _Rule(RegExp(r"'[^'\n]*'"), string),
        _Rule(RegExp(r'\b(true|false|null|yes|no|on|off)\b'), keyword),
        _Rule(RegExp(r'(?<![\w.])-?\d+(\.\d+)?(?![\w.])'), number),
      ],
      CodeLanguage.toml => [
        _Rule(RegExp(r'#.*$', multiLine: true), comment),
        _Rule(RegExp(r'^\s*\[[^\]\n]+\]', multiLine: true), keyword),
        _Rule(RegExp(r'^\s*[\w.\-"]+(?=\s*=)', multiLine: true), key),
        _Rule(RegExp(r'"[^"\n]*"'), string),
        _Rule(RegExp(r'\b(true|false)\b'), keyword),
        _Rule(RegExp(r'(?<![\w.])-?\d+(\.\d+)?(?![\w.])'), number),
      ],
      CodeLanguage.properties => [
        _Rule(RegExp(r'^\s*[#!].*$', multiLine: true), comment),
        _Rule(RegExp(r'^[^=#!\n]+(?==)', multiLine: true), key),
        _Rule(RegExp(r'\b(true|false)\b'), keyword),
        _Rule(RegExp(r'(?<==)-?\d+$', multiLine: true), number),
      ],
      CodeLanguage.json => [
        _Rule(RegExp(r'"(?:[^"\\\n]|\\.)*"(?=\s*:)'), key),
        _Rule(RegExp(r'"(?:[^"\\\n]|\\.)*"'), string),
        _Rule(RegExp(r'\b(true|false|null)\b'), keyword),
        _Rule(RegExp(r'-?\d+(\.\d+)?([eE][+-]?\d+)?'), number),
      ],
      CodeLanguage.plain => const [],
    };
  }

  @override
  TextSpan buildTextSpan({required BuildContext context, TextStyle? style, required bool withComposing}) {
    final rules = _rules(context.voxel);
    if (rules.isEmpty || text.length > 400000) {
      return TextSpan(style: style, text: text);
    }
    // The first rule that claims a character wins, so comments hide keys inside them.
    final owners = List<TextStyle?>.filled(text.length, null);
    for (final rule in rules) {
      for (final match in rule.pattern.allMatches(text)) {
        for (var index = match.start; index < match.end; index++) {
          owners[index] ??= rule.style;
        }
      }
    }
    final spans = <TextSpan>[];
    var start = 0;
    for (var index = 1; index <= text.length; index++) {
      if (index == text.length || owners[index] != owners[start]) {
        spans.add(TextSpan(text: text.substring(start, index), style: owners[start]));
        start = index;
      }
    }
    return TextSpan(style: style, children: spans);
  }
}

class CodeEditor extends StatelessWidget {
  const CodeEditor({super.key, required this.controller, this.readOnly = false, this.onChanged});

  final HighlightingController controller;
  final bool readOnly;
  final ValueChanged<String>? onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = context.voxel;
    return Container(
      decoration: BoxDecoration(color: colors.console, borderRadius: BorderRadius.circular(12), border: Border.all(color: colors.cardBorder)),
      child: TextField(
        controller: controller,
        readOnly: readOnly,
        onChanged: onChanged,
        maxLines: null,
        expands: true,
        textAlignVertical: TextAlignVertical.top,
        keyboardType: TextInputType.multiline,
        style: const TextStyle(fontFamily: 'monospace', fontFamilyFallback: ['Consolas', 'Menlo', 'DejaVu Sans Mono'], fontSize: 13, height: 1.4, color: Color(0xFFDDDDDD)),
        decoration: const InputDecoration(filled: false, border: InputBorder.none, enabledBorder: InputBorder.none, focusedBorder: InputBorder.none, contentPadding: EdgeInsets.all(12)),
      ),
    );
  }
}
