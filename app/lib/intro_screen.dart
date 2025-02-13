// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/theme/theme.dart';

import 'navigation/navigation.dart';

class IntroScreen extends StatelessWidget {
  const IntroScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final isUserLoading = context.select((LoadableUserCubit cubit) {
      return cubit.state is LoadingUser;
    });

    return Scaffold(
      body: Center(
        child: Container(
          height: MediaQuery.of(context).size.height,
          padding: const EdgeInsets.fromLTRB(20, 100, 20, 50),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            mainAxisSize: MainAxisSize.max,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Image(
                image: const AssetImage('assets/images/logo.png'),
                height: 100,
                filterQuality: FilterQuality.high,
                color: Colors.grey[350],
              ),
              _GradientText(
                "Prototype.",
                gradient: const LinearGradient(
                  colors: [
                    Color.fromARGB(255, 34, 163, 255),
                    Color.fromARGB(255, 72, 23, 250)
                  ],
                  transform: GradientRotation(1.1),
                ),
                style: const TextStyle(
                  fontSize: 36,
                  letterSpacing: -0.9,
                ).merge(VariableFontWeight.medium),
              ),
              // Text button that opens the developer settings screen
              TextButton(
                onPressed: () =>
                    context.read<NavigationCubit>().openDeveloperSettings(),
                style: textButtonStyle(context),
                child: const Text('Developer Settings'),
              ),
              if (!isUserLoading)
                Column(
                  crossAxisAlignment: isSmallScreen(context)
                      ? CrossAxisAlignment.stretch
                      : CrossAxisAlignment.center,
                  children: [
                    OutlinedButton(
                      onPressed: () =>
                          context.read<NavigationCubit>().openServerChoice(),
                      style: buttonStyle(context, true),
                      child: const Text('Sign up'),
                    )
                  ],
                )
            ],
          ),
        ),
      ),
    );
  }
}

class _GradientText extends StatelessWidget {
  const _GradientText(
    this.text, {
    required this.gradient,
    this.style,
  });

  final String text;
  final TextStyle? style;
  final Gradient gradient;

  @override
  Widget build(BuildContext context) {
    return ShaderMask(
      blendMode: BlendMode.srcIn,
      shaderCallback: (bounds) => gradient.createShader(
        Rect.fromLTWH(0, 0, bounds.width, bounds.height),
      ),
      child: Text(text, style: style),
    );
  }
}
