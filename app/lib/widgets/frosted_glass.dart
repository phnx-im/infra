// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:ui';

import 'package:flutter/material.dart';

class FrostedGlass extends StatelessWidget {
  const FrostedGlass({
    super.key,
    required this.child,
    this.borderRadius,
    this.enabled = true,
  });

  final BorderRadiusGeometry? borderRadius;
  final Widget child;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return enabled
        ? ClipRRect(
          borderRadius: borderRadius ?? BorderRadius.zero,
          child: BackdropFilter(
            filter: ImageFilter.blur(
              sigmaX: 32,
              sigmaY: 32,
              tileMode: TileMode.repeated,
            ),
            child: child,
          ),
        )
        : child;
  }
}
