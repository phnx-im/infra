// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';

/// Global configuration of the app
///
/// Can be accessed with [AppConfig.of] from anywhere in the app.
class AppConfig extends InheritedWidget {
  const AppConfig({
    super.key,

    this.frostedGlassEnabled = true,
    required super.child,
  });

  /// Enables or disabled the frosted glass effect.
  ///
  /// Useful for disabling the effect in golden tests, because the effect renders differently
  /// on different platforms.
  final bool frostedGlassEnabled;

  static AppConfig of(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<AppConfig>()!;
  }

  /// Configuration never changes
  @override
  bool updateShouldNotify(AppConfig oldWidget) => false;
}
