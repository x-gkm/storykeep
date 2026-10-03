import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../api/models.dart';
import '../../util/format.dart';

/// A simple time-scaled line chart for one measurement series.
class MeasurementChart extends StatelessWidget {
  const MeasurementChart({super.key, required this.points, required this.unit, this.height = 220});

  final List<SeriesPoint> points;
  final String unit;
  final double height;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (points.isEmpty) {
      return SizedBox(
        height: height,
        child: Center(child: Text('No data to chart yet', style: theme.textTheme.bodyMedium)),
      );
    }
    return Semantics(
      label: 'Chart of ${points.length} measurements in $unit, '
          'from ${formatNumber(points.first.value)} to ${formatNumber(points.last.value)}',
      child: SizedBox(
        height: height,
        width: double.infinity,
        child: CustomPaint(
          painter: _ChartPainter(
            points: points,
            unit: unit,
            line: theme.colorScheme.primary,
            grid: theme.colorScheme.outlineVariant,
            label: theme.textTheme.labelSmall!.copyWith(color: theme.colorScheme.onSurfaceVariant),
          ),
        ),
      ),
    );
  }
}

class _ChartPainter extends CustomPainter {
  _ChartPainter({required this.points, required this.unit, required this.line, required this.grid, required this.label});

  final List<SeriesPoint> points;
  final String unit;
  final Color line;
  final Color grid;
  final TextStyle label;

  static const _left = 48.0;
  static const _bottom = 28.0;
  static const _top = 12.0;
  static const _right = 12.0;

  TextPainter _text(String s) => TextPainter(text: TextSpan(text: s, style: label), textDirection: TextDirection.ltr)..layout();

  @override
  void paint(Canvas canvas, Size size) {
    final plot = Rect.fromLTRB(_left, _top, size.width - _right, size.height - _bottom);
    var minV = points.map((p) => p.value).reduce(math.min);
    var maxV = points.map((p) => p.value).reduce(math.max);
    if (minV == maxV) {
      minV -= 1;
      maxV += 1;
    }
    final pad = (maxV - minV) * 0.1;
    minV = math.max(0, minV - pad);
    maxV += pad;

    final t0 = points.first.date.millisecondsSinceEpoch.toDouble();
    var t1 = points.last.date.millisecondsSinceEpoch.toDouble();
    if (t1 == t0) t1 = t0 + 1;

    Offset at(SeriesPoint p) => Offset(
          points.length == 1 ? plot.center.dx : plot.left + (p.date.millisecondsSinceEpoch - t0) / (t1 - t0) * plot.width,
          plot.bottom - (p.value - minV) / (maxV - minV) * plot.height,
        );

    final gridPaint = Paint()
      ..color = grid
      ..strokeWidth = 1;
    const rows = 4;
    for (var i = 0; i <= rows; i++) {
      final y = plot.bottom - plot.height * i / rows;
      canvas.drawLine(Offset(plot.left, y), Offset(plot.right, y), gridPaint);
      final tp = _text(formatNumber(minV + (maxV - minV) * i / rows));
      tp.paint(canvas, Offset(plot.left - tp.width - 6, y - tp.height / 2));
    }
    final unitTp = _text(unit);
    unitTp.paint(canvas, Offset(0, plot.bottom + 8));

    final first = _text(formatDate(points.first.date));
    first.paint(canvas, Offset(plot.left, plot.bottom + 8));
    if (points.length > 1) {
      final last = _text(formatDate(points.last.date));
      last.paint(canvas, Offset(plot.right - last.width, plot.bottom + 8));
    }

    final path = Path();
    for (var i = 0; i < points.length; i++) {
      final o = at(points[i]);
      i == 0 ? path.moveTo(o.dx, o.dy) : path.lineTo(o.dx, o.dy);
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = line
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.5
        ..strokeJoin = StrokeJoin.round,
    );
    final dot = Paint()..color = line;
    for (final p in points) {
      canvas.drawCircle(at(p), 4, dot);
    }
  }

  @override
  bool shouldRepaint(_ChartPainter old) =>
      old.points != points || old.line != line || old.grid != grid || old.unit != unit;
}
