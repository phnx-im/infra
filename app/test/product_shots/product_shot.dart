// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';
import 'dart:math';

import 'package:device_frame_plus/device_frame_plus.dart';
import 'package:flutter/material.dart';

class ProductShot extends StatelessWidget {
  const ProductShot({
    super.key,
    required this.widthPx,
    required this.heightPx,
    required this.backgroundColor,
    required this.label,
    required this.child,
    this.device,
  });

  final int widthPx;
  final int heightPx;
  final Color backgroundColor;
  final String label;
  final DeviceInfo? device;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final dev =
        device ??
        (Platform.isAndroid ? Devices.android.pixel4 : Devices.ios.iPhone13);
    final scale = Platform.isAndroid ? 1.5 : 1.0;

    return Center(
      child: SizedBox(
        width: widthPx.toDouble(),
        height: heightPx.toDouble(),
        child: DecoratedBox(
          // Light metallic grey effect using a subtle vertical gradient.
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: [
                // Slightly cooler (more blue) metallic greys
                Color.lerp(backgroundColor, const Color(0xFFD5DFEA), 0.4)!,
                Color.lerp(backgroundColor, const Color(0xFFC7D3E0), 0.25)!,
                Color.lerp(backgroundColor, const Color(0xFFDBE4EE), 0.6)!,
              ],
            ),
          ),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final frameMaxWidth = constraints.maxWidth * 0.80;
              final topSpacer = constraints.maxHeight * 0.14;
              final frameDownwardOffset =
                  constraints.maxHeight * (Platform.isAndroid ? 0.30 : 0.20);
              return Padding(
                padding: const EdgeInsets.all(32),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    SizedBox(height: topSpacer),
                    _ShotTitle(text: label),
                    const SizedBox(height: 56),
                    Expanded(
                      child: Transform.translate(
                        offset: Offset(0, frameDownwardOffset),
                        child: Align(
                          alignment: Alignment.topCenter,
                          child: ConstrainedBox(
                            constraints: BoxConstraints(
                              maxWidth: frameMaxWidth,
                              maxHeight: constraints.maxHeight * 0.80,
                            ),
                            child: FittedBox(
                              fit: BoxFit.fitWidth,
                              alignment: Alignment.topCenter,
                              child: RepaintBoundary(
                                key: const ValueKey('frame'),
                                child: Transform.scale(
                                  scale: scale,
                                  child: DeviceFrame(
                                    device: dev,
                                    isFrameVisible: true,
                                    orientation: Orientation.portrait,
                                    screen: Stack(
                                      children: [
                                        child,
                                        const _Header(color: Colors.black),
                                      ],
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      height: 40,
      child: Padding(
        padding: const EdgeInsets.only(left: 48.0, top: 8, right: 23),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            _IosTime(color: color),
            Column(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                Row(
                  spacing: 5.5,
                  children: [
                    _IosSignalStrength(color: color),
                    _IosWifiSymbol(color: color),
                    _IosBattery(color: color),
                  ],
                ),
                const SizedBox(height: 6),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _IosTime extends StatelessWidget {
  const _IosTime({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return Text(
      "9:41",
      style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold, color: color),
    );
  }
}

class _IosSignalStrength extends StatelessWidget {
  const _IosSignalStrength({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    const height = 12.0;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      spacing: height / 6,
      children: [
        _IosSignalBar(color: color, height: height, fraction: 0.4),
        _IosSignalBar(color: color, height: height, fraction: 0.6),
        _IosSignalBar(color: color, height: height, fraction: 0.8),
        _IosSignalBar(color: color, height: height, fraction: 1.0),
      ],
    );
  }
}

class _IosSignalBar extends StatelessWidget {
  const _IosSignalBar({
    required this.color,
    required this.height,
    required this.fraction,
  });

  final Color color;
  final double height;
  final double fraction;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: height / 4,
      height: height * fraction,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(height / 15),
        color: color,
      ),
    );
  }
}

class _IosWifiSymbol extends StatelessWidget {
  const _IosWifiSymbol({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 22,
      height: 14,
      child: CustomPaint(painter: _WifiPainter(color: color, strokeWidth: 2.8)),
    );
  }
}

class _WifiPainter extends CustomPainter {
  const _WifiPainter({required this.color, required this.strokeWidth});

  final Color color;
  final double strokeWidth;

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height - strokeWidth / 1.5);
    const double startAngle = -pi / 2 - pi / 4;
    const double sweepAngle = pi / 2;

    final paint =
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = strokeWidth
          ..strokeCap = StrokeCap.square;
    final maxR = size.height;

    for (double i = 0.01; i <= .4; i += .2) {
      final r = maxR * (i / 3);
      final rect = Rect.fromCircle(center: center, radius: r);
      canvas.drawArc(rect, startAngle, sweepAngle, false, paint);
    }

    for (double i = 1; i <= 2; i++) {
      final r = maxR * (i / 3) + 1;
      final rect = Rect.fromCircle(center: center, radius: r);
      canvas.drawArc(rect, startAngle, sweepAngle, false, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _WifiPainter oldDelegate) {
    return oldDelegate.color != color || oldDelegate.strokeWidth != strokeWidth;
  }
}

class _IosBattery extends StatelessWidget {
  const _IosBattery({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    const width = 25.0;
    const height = 14.0;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      spacing: width / 20,
      children: [
        Container(
          width: width,
          height: height,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(width / 6),
            border: Border.all(color: color.withAlpha(127), width: width / 20),
          ),
          child: Padding(
            padding: const EdgeInsets.all(width / 22),
            child: Container(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(width / 10),
                color: color,
              ),
              width: width * 0.9,
              height: height * 0.9,
            ),
          ),
        ),
        SizedBox(
          width: width / 20,
          height: height / 3.5,
          child: CustomPaint(
            painter: _BatteryCapPainter(color: color.withAlpha(127)),
          ),
        ),
      ],
    );
  }
}

class _BatteryCapPainter extends CustomPainter {
  const _BatteryCapPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) {
      return;
    }

    final paint =
        Paint()
          ..color = color
          ..style = PaintingStyle.fill;

    final rect = Offset.zero & size;
    final radius = Radius.circular(size.width / 1.2);
    final rrect = RRect.fromRectAndCorners(
      rect,
      topRight: radius,
      bottomRight: radius,
    );

    canvas.drawRRect(rrect, paint);
  }

  @override
  bool shouldRepaint(covariant _BatteryCapPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}

/// Large store headline used in product shots.
class _ShotTitle extends StatelessWidget {
  const _ShotTitle({required this.text});

  final String text;

  static const _style = TextStyle(
    fontSize: 64,
    fontWeight: FontWeight.w800,
    color: Color.fromARGB(255, 59, 61, 65), // dark grey title
    height: 1.5,
    letterSpacing: -0.5,
  );

  @override
  Widget build(BuildContext context) {
    return DefaultTextStyle.merge(
      style: _style,
      child: Text(
        text,
        maxLines: 2,
        textAlign: TextAlign.center,
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
