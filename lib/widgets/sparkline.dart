// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:flutter/material.dart';

/// Line chart of percentages (0-100).
class Sparkline extends StatelessWidget {
  const Sparkline({super.key, required this.samples, required this.color, required this.idleColor, this.height = 36});

  final List<double> samples;
  final Color color;
  final Color idleColor;
  final double height;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      width: double.infinity,
      child: CustomPaint(painter: _SparkPainter(samples, color, idleColor)),
    );
  }
}

class _SparkPainter extends CustomPainter {
  _SparkPainter(this.samples, this.color, this.idleColor);

  final List<double> samples;
  final Color color;
  final Color idleColor;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;
    if (samples.length < 2) {
      canvas.drawLine(Offset(0, size.height / 2), Offset(size.width, size.height / 2), paint..color = idleColor);
      return;
    }
    final path = Path();
    final fill = Path()..moveTo(0, size.height);
    for (var i = 0; i < samples.length; i++) {
      final x = size.width * i / (samples.length - 1);
      final y = size.height - (samples[i].clamp(0, 100) / 100) * size.height;
      i == 0 ? path.moveTo(x, y) : path.lineTo(x, y);
      fill.lineTo(x, y);
    }
    fill
      ..lineTo(size.width, size.height)
      ..close();
    canvas.drawPath(fill, Paint()..color = color.withValues(alpha: 0.15));
    canvas.drawPath(path, paint..color = color);
  }

  @override
  bool shouldRepaint(covariant _SparkPainter oldDelegate) =>
      oldDelegate.samples != samples || oldDelegate.color != color || oldDelegate.idleColor != idleColor;
}
