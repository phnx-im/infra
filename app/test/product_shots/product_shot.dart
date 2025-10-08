// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'android_status_bar.dart';
import 'ios_status_bar.dart';
import 'product_shot_frame.dart';
import 'product_shot_device.dart';

class ProductShot extends StatelessWidget {
  const ProductShot({
    super.key,
    required this.size,
    required this.backgroundColor,
    required this.header,
    required this.subheader,
    required this.child,
    this.device,
  });

  final Size size;
  final Color backgroundColor;
  final String header;
  final String subheader;
  final ProductShotDevice? device;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final platform = device?.platform ?? _defaultPlatform();
    final dev = device ?? ProductShotDevices.forPlatform(platform);
    final frameStyle = _frameStyleFor(dev.platform);
    final scale = _deviceScale(dev.platform);
    final statusBarHeight = _statusBarHeightFor(dev);
    final statusBar = _statusBarFor(dev.platform, statusBarHeight);
    final resolvedSafeArea = EdgeInsets.only(
      left: dev.safeArea.left,
      top: math.max(dev.safeArea.top, statusBarHeight),
      right: dev.safeArea.right,
      bottom: dev.safeArea.bottom,
    );

    return Center(
      child: SizedBox(
        width: size.width,
        height: size.height,
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
                  constraints.maxHeight * frameStyle.verticalOffsetFactor;
              return Padding(
                padding: const EdgeInsets.all(32),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    SizedBox(height: topSpacer),
                    _ShotTitle(text: header),
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
                                  child: ProductShotFrame(
                                    statusBar: statusBar,
                                    statusBarHeight: statusBarHeight,
                                    screenSize: dev.screenSize,
                                    devicePixelRatio: dev.pixelRatio,
                                    safeArea: resolvedSafeArea,
                                    borderWidth: frameStyle.borderWidth,
                                    cornerRadius: frameStyle.cornerRadius,
                                    frameColor: frameStyle.frameColor,
                                    screenBackgroundColor:
                                        frameStyle.screenBackgroundColor,
                                    child: child,
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

ProductShotPlatform _defaultPlatform() {
  return Platform.isAndroid
      ? ProductShotPlatform.android
      : ProductShotPlatform.ios;
}

double _deviceScale(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return 1.15;
    case ProductShotPlatform.ios:
      return 1.15;
    case ProductShotPlatform.macos:
    case ProductShotPlatform.windows:
    case ProductShotPlatform.linux:
      return 1.0;
  }
}

_FrameStyle _frameStyleFor(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return const _FrameStyle(
        borderWidth: 20,
        cornerRadius: 48,
        frameColor: Color(0xFF1C262F),
        screenBackgroundColor: Colors.black,
        verticalOffsetFactor: 0.18,
      );
    case ProductShotPlatform.ios:
      return const _FrameStyle(
        borderWidth: 18,
        cornerRadius: 64,
        frameColor: Color(0xFFF2F4F6),
        screenBackgroundColor: Colors.black,
        verticalOffsetFactor: 0.18,
      );
    case ProductShotPlatform.macos:
      return const _FrameStyle(
        borderWidth: 28,
        cornerRadius: 48,
        frameColor: Color(0xFF1F1F23),
        screenBackgroundColor: Color(0xFF121212),
        verticalOffsetFactor: 0.18,
      );
    case ProductShotPlatform.windows:
    case ProductShotPlatform.linux:
      return const _FrameStyle();
  }
}

double _statusBarHeightFor(ProductShotDevice device) {
  if (device.statusBarHeight != null) {
    return device.statusBarHeight!;
  }
  if (device.safeArea.top > 0) {
    return device.safeArea.top;
  }

  switch (device.platform) {
    case ProductShotPlatform.android:
      return 40.0;
    case ProductShotPlatform.ios:
      return 44.0;
    case ProductShotPlatform.macos:
    case ProductShotPlatform.windows:
    case ProductShotPlatform.linux:
      return 0.0;
  }
}

Widget _statusBarFor(ProductShotPlatform platform, double statusBarHeight) {
  switch (platform) {
    case ProductShotPlatform.android:
      return AndroidStatusBar(height: statusBarHeight);
    case ProductShotPlatform.ios:
      return IosStatusBar(height: statusBarHeight);
    case ProductShotPlatform.macos:
    case ProductShotPlatform.windows:
    case ProductShotPlatform.linux:
      return const SizedBox.shrink();
  }
}

class _FrameStyle {
  const _FrameStyle({
    this.borderWidth = 32,
    this.cornerRadius = 80,
    this.frameColor = Colors.black,
    this.screenBackgroundColor = Colors.black,
    this.verticalOffsetFactor = 0.20,
  });

  final double borderWidth;
  final double cornerRadius;
  final Color frameColor;
  final Color screenBackgroundColor;
  final double verticalOffsetFactor;
}
