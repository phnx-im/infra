import 'dart:math';

import 'package:device_frame_plus/device_frame_plus.dart';
import 'package:flutter/material.dart';

import 'product_shots.dart';

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
    final dev = device ?? Devices.ios.iPhone13;
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
              final frameDownwardOffset = constraints.maxHeight * 0.20;
              return Padding(
                padding: const EdgeInsets.all(32),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    SizedBox(height: topSpacer),
                    ShotTitle(text: label),
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
        padding: const EdgeInsets.only(left: 48.0, top: 8, right: 28),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            _IosTime(color: color),
            Column(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                Row(
                  spacing: 6,
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
      style: TextStyle(
        fontSize: 18,
        fontWeight: FontWeight.normal,
        color: color,
      ),
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
      spacing: height / 8,
      children: [
        Container(width: height / 4, height: height * 0.4, color: color),
        Container(width: height / 4, height: height * 0.6, color: color),
        Container(width: height / 4, height: height * 0.8, color: color),
        Container(
          width: height / 4,
          height: height * 1,
          color: color.withAlpha(100),
        ),
      ],
    );
  }
}

class _IosWifiSymbol extends StatelessWidget {
  const _IosWifiSymbol({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 24,
      height: 12,
      child: CustomPaint(painter: _WifiPainter(color: color, strokeWidth: 3)),
    );
  }
}

class _WifiPainter extends CustomPainter {
  const _WifiPainter({required this.color, required this.strokeWidth});

  final Color color;
  final double strokeWidth;

  @override
  void paint(Canvas canvas, Size size) {
    final paint =
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = strokeWidth
          ..strokeCap = StrokeCap.square;

    final center = Offset(size.width / 2, size.height - strokeWidth / 1.5);
    const double startAngle = -pi / 2 - pi / 4;
    const double sweepAngle = pi / 2;

    final maxR = size.height;

    for (double i = 0.1; i <= 3; i++) {
      final r = maxR * (i / 3);
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
    const width = 22.0;
    const height = 12.0;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Container(
          width: width,
          height: height,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(width / 10),
            border: Border.all(color: color, width: width / 20),
          ),
          child: Padding(
            padding: const EdgeInsets.all(width / 20),
            child: Container(
              width: width * 0.9,
              height: height * 0.9,
              color: color,
            ),
          ),
        ),
        Container(width: width / 20, height: height / 3, color: color),
      ],
    );
  }
}
