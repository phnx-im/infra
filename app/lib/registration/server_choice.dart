// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'registration_cubit.dart';

class ServerChoice extends StatelessWidget {
  const ServerChoice({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      resizeToAvoidBottomInset: false,
      appBar: AppBar(
        title: const Text('Sign up'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
      ),
      body: SafeArea(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: Spacings.s),
          child: Center(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                const Text(
                  'Choose a server where you want to create your account',
                ),
                Form(
                  autovalidateMode: AutovalidateMode.always,
                  child: Column(
                    children: [
                      const SizedBox(height: 10),
                      ConstrainedBox(
                        constraints: BoxConstraints.tight(const Size(300, 80)),
                        child: TextFormField(
                          autofocus:
                              (Platform.isIOS || Platform.isAndroid)
                                  ? false
                                  : true,
                          decoration: const InputDecoration(
                            hintText: 'DOMAIN NAME',
                          ),
                          initialValue:
                              context.read<RegistrationCubit>().state.domain,
                          style: inputTextStyle,
                          onChanged: (String value) {
                            context.read<RegistrationCubit>().setDomain(value);
                          },
                        ),
                      ),
                    ],
                  ),
                ),
                Column(
                  crossAxisAlignment:
                      isSmallScreen(context)
                          ? CrossAxisAlignment.stretch
                          : CrossAxisAlignment.center,
                  children: const [_NextButton()],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _NextButton extends StatelessWidget {
  const _NextButton();

  @override
  Widget build(BuildContext context) {
    final isDomainValid = context.select(
      (RegistrationCubit cubit) => cubit.state.isDomainValid,
    );
    return OutlinedButton(
      onPressed:
          isDomainValid
              ? () => context.read<NavigationCubit>().openIntroScreen(
                IntroScreenType.displayNamePicture,
              )
              : null,
      style: buttonStyle(context, isDomainValid),
      child: const Text('Next'),
    );
  }
}
