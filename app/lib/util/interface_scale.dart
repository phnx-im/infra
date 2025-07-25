// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:prototype/user/user.dart';
import 'package:provider/provider.dart';

/// Scales the child's interface by keeping the same size
///
/// The scale factor is taken from the [`UserSettingsCubit`].
class InterfaceScale extends StatelessWidget {
  const InterfaceScale({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final factor = context.select(
      (UserSettingsCubit cubit) => cubit.state.interfaceScale,
    );

    // remove text scaling on Linux
    final wrappedChild =
        (Platform.isLinux)
            ? MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(textScaler: const TextScaler.linear(1.0)),
              child: child,
            )
            : child;
    return factor == 1.0
        ? wrappedChild
        : FractionallySizedBox(
          widthFactor: 1 / factor,
          heightFactor: 1 / factor,
          child: Transform.scale(scale: factor, child: wrappedChild),
        );
  }
}
