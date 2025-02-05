import 'dart:io';

import 'package:flutter/widgets.dart';

/// Scales the child's interface by keeping the same size
class InterfaceScale extends StatelessWidget {
  const InterfaceScale({
    required this.factor,
    required this.child,
    super.key,
  });

  final double factor;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    // remove text scaling on Linux
    final wrappedChild = (Platform.isLinux)
        ? MediaQuery(
            data: MediaQuery.of(context).copyWith(
              textScaler: const TextScaler.linear(1.0),
            ),
            child: child,
          )
        : child;
    return factor == 1.0
        ? wrappedChild
        : FractionallySizedBox(
            widthFactor: 1 / factor,
            heightFactor: 1 / factor,
            child: Transform.scale(
              scale: factor,
              child: wrappedChild,
            ),
          );
  }
}
