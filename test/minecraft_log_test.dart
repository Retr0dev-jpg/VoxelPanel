import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:voxel_panel/widgets/minecraft_log.dart';

void main() {
  test('ansi paper help uses the minecraft palette', () {
    final spans = minecraftLogSpans('\x1B[93mHelp: \x1B[97mIndex\x1B[0m');
    expect(spans.map((span) => span.text).toList(), ['Help: ', 'Index']);
    expect(spans[0].style?.color, const Color(0xFFFFFF55));
    expect(spans[1].style?.color, const Color(0xFFFFFFFF));
  });

  test('section codes reset style when the color changes', () {
    final spans = minecraftLogSpans('§c§lErrore§r ok');
    expect(spans.map((span) => span.text).join(), 'Errore ok');
    expect(spans[0].style?.color, const Color(0xFFFF5555));
    expect(spans[0].style?.fontWeight, FontWeight.w700);
    expect(spans[1].style?.color, isNull);
  });
}
