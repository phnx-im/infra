// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/widgets/widgets.dart';

import 'registration_cubit.dart';

class UsernameChoice extends StatelessWidget {
  const UsernameChoice({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      resizeToAvoidBottomInset: false,
      appBar: AppBar(
        title: const Text('Sign up', style: TextStyle(fontFamily: fontFamily)),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
      ),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: Spacings.s),
          child: Center(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                const Text('Choose a username'),
                Form(
                  autovalidateMode: AutovalidateMode.always,
                  child: Column(
                    children: [
                      const SizedBox(height: 5),
                      ConstrainedBox(
                        constraints: BoxConstraints.tight(const Size(300, 80)),
                        child: const _UsernameTextField(),
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

class _UsernameTextField extends StatelessWidget {
  const _UsernameTextField();

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      autofocus: (Platform.isIOS || Platform.isAndroid) ? false : true,
      initialValue: context.read<RegistrationCubit>().state.username,
      decoration: const InputDecoration(hintText: 'USERNAME'),
      style: inputTextStyle,
      validator: _usernameValidator,
      onChanged: (String value) {
        context.read<RegistrationCubit>().setUsername(value);
      },
    );
  }

  String? _usernameValidator(String? value) {
    // alphanumeric
    final validCharacters = RegExp(r'^[a-zA-Z0-9@.]+$');
    var containsInvalidChars =
        value!.isNotEmpty && !validCharacters.hasMatch(value);
    var isTooLong = value.length >= 64;
    var isTooShort = value.isEmpty;
    var hasRightLength = !isTooShort && !isTooLong;
    if (hasRightLength && !containsInvalidChars) {
      return null;
    } else if (containsInvalidChars) {
      return 'Please use alphanumeric characters only';
    } else if (isTooLong) {
      return 'Maximum length is 64 characters';
    }
    return null;
  }
}

class _NextButton extends StatelessWidget {
  const _NextButton();

  @override
  Widget build(BuildContext context) {
    final isActive = context.select(
      (RegistrationCubit cubit) => cubit.state.isUsernameValid,
    );
    return OutlinedButton(
      style: buttonStyle(context, isActive),
      onPressed:
          isActive
              ? () => context.read<NavigationCubit>().openIntroScreen(
                IntroScreenType.displayNamePicture,
              )
              : null,
      child: const Text('Next'),
    );
  }
}
