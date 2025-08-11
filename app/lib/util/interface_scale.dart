// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/widgets.dart';
import 'package:prototype/ui/theme/scale.dart';
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
    final userUiFactor = context.select(
      (UserSettingsCubit cubit) => cubit.state.interfaceScale,
    );

    final scalingFactors = getScalingFactors(context);
    final uiScalingFactor = scalingFactors.uiFactor * userUiFactor;

    // remove text scaling on Linux
    final wrappedChild = MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(textScaler: TextScaler.linear(scalingFactors.textFactor)),
      child: child,
    );
    return uiScalingFactor == 1.0
        ? wrappedChild
        : FractionallySizedBox(
          widthFactor: 1 / uiScalingFactor,
          heightFactor: 1 / uiScalingFactor,
          child: Transform.scale(scale: uiScalingFactor, child: wrappedChild),
        );
  }
}
