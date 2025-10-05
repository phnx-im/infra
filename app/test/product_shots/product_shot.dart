// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:device_frame_plus/device_frame_plus.dart';
import 'package:flutter/material.dart';
import 'android_status_bar.dart';
import 'ios_status_bar.dart';

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
    final statusBar =
        Platform.isAndroid ? const AndroidStatusBar() : const IosStatusBar();

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
                                      clipBehavior: Clip.hardEdge,
                                      children: [child, statusBar],
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
