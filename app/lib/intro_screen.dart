// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user.dart';
import 'package:air/theme/theme.dart';

class IntroScreen extends StatelessWidget {
  const IntroScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final isUserLoading = context.select((LoadableUserCubit cubit) {
      return cubit.state is LoadingUser;
    });

    final loc = AppLocalizations.of(context);

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
                loc.appTitle,
                gradient: const LinearGradient(
                  colors: [
                    Color.fromARGB(255, 34, 163, 255),
                    Color.fromARGB(255, 72, 23, 250),
                  ],
                  transform: GradientRotation(1.1),
                ),
                style: const TextStyle(
                  fontSize: 36,
                  letterSpacing: -0.9,
                  fontWeight: FontWeight.bold,
                ),
              ),
              // Text button that opens the developer settings screen
              TextButton(
                onPressed:
                    () =>
                        context.read<NavigationCubit>().openDeveloperSettings(),
                style: dynamicTextButtonStyle(context, true, true),
                child: Text(loc.settings_developerSettings),
              ),
              if (!isUserLoading)
                Column(
                  crossAxisAlignment:
                      isSmallScreen(context)
                          ? CrossAxisAlignment.stretch
                          : CrossAxisAlignment.center,
                  children: [
                    OutlinedButton(
                      onPressed:
                          () =>
                              context
                                  .read<NavigationCubit>()
                                  .openServerChoice(),
                      style: buttonStyle(CustomColorScheme.of(context), true),
                      child: Text(loc.introScreen_signUp),
                    ),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _GradientText extends StatelessWidget {
  const _GradientText(this.text, {required this.gradient, this.style});

  final String text;
  final TextStyle? style;
  final Gradient gradient;

  @override
  Widget build(BuildContext context) {
    return ShaderMask(
      blendMode: BlendMode.srcIn,
      shaderCallback:
          (bounds) => gradient.createShader(
            Rect.fromLTWH(0, 0, bounds.width, bounds.height),
          ),
      child: Text(text, style: style),
    );
  }
}
