// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:air/core/core.dart';
import 'package:air/main.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'registration_cubit.dart';

class DisplayNameAvatarChoice extends StatelessWidget {
  const DisplayNameAvatarChoice({super.key});

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
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: Spacings.s),
          child: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                const _UserAvatarPicker(),
                Column(
                  children: [
                    const Text('Choose a picture and a display name'),
                    const SizedBox(height: 20),
                    Form(
                      autovalidateMode: AutovalidateMode.always,
                      child: ConstrainedBox(
                        constraints: BoxConstraints.tight(const Size(300, 80)),
                        child: const _DisplayNameTextField(),
                      ),
                    ),
                  ],
                ),
                const _SignUpFooter(),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _DisplayNameTextField extends StatelessWidget {
  const _DisplayNameTextField();

  @override
  Widget build(BuildContext context) {
    final displayName = context.select(
      (RegistrationCubit cubit) => cubit.state.displayName,
    );

    return TextFormField(
      autofocus: isSmallScreen(context) ? false : true,
      decoration: const InputDecoration(hintText: 'DISPLAY NAME'),
      initialValue: displayName,
      onChanged: (value) {
        context.read<RegistrationCubit>().setDisplayName(value);
      },
    );
  }
}

class _UserAvatarPicker extends StatelessWidget {
  const _UserAvatarPicker();

  @override
  Widget build(BuildContext context) {
    final (displayName, avatar) = context.select(
      (RegistrationCubit cubit) => (
        cubit.state.displayName,
        cubit.state.avatar,
      ),
    );

    return UserAvatar(
      displayName: displayName,
      image: avatar,
      size: 100,
      onPressed: () async {
        var registrationCubit = context.read<RegistrationCubit>();
        // Image picker
        final ImagePicker picker = ImagePicker();
        // Pick an image.
        final XFile? image = await picker.pickImage(
          source: ImageSource.gallery,
        );
        final bytes = await image?.readAsBytes();
        registrationCubit.setAvatar(bytes?.toImageData());
      },
    );
  }
}

class _SignUpFooter extends StatelessWidget {
  const _SignUpFooter();

  @override
  Widget build(BuildContext context) {
    final isSigningUp = context.select(
      (RegistrationCubit cubit) => cubit.state.isSigningUp,
    );

    return Column(
      crossAxisAlignment:
          isSmallScreen(context)
              ? CrossAxisAlignment.stretch
              : CrossAxisAlignment.center,
      children: [
        if (!isSigningUp)
          OutlinedButton(
            onPressed:
                !isSigningUp
                    ? () async {
                      final navigationCubit = context.read<NavigationCubit>();
                      final error =
                          await context.read<RegistrationCubit>().signUp();
                      if (error == null) {
                        navigationCubit.openHome();
                      } else if (context.mounted) {
                        showErrorBanner(context, error.message);
                      }
                    }
                    : null,
            style: buttonStyle(CustomColorScheme.of(context), !isSigningUp),
            child: const Text('Sign up'),
          ),
        if (isSigningUp)
          Align(
            child: CircularProgressIndicator(
              value: null,
              valueColor: AlwaysStoppedAnimation<Color>(
                CustomColorScheme.of(context).text.secondary,
              ),
              backgroundColor: Colors.transparent,
            ),
          ),
      ],
    );
  }
}
