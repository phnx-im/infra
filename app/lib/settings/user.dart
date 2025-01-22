// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/main.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

class UserSettingsScreen extends StatefulWidget {
  const UserSettingsScreen({super.key});

  @override
  State<UserSettingsScreen> createState() => _UserSettingsScreenState();
}

class _UserSettingsScreenState extends State<UserSettingsScreen> {
  String? newDisplayName;
  Uint8List? newProfilePicture;

  bool get _isChanged => newDisplayName != null || newProfilePicture != null;

  void _save(BuildContext context) async {
    final user = context.read<UserCubit>();
    final messenger = ScaffoldMessenger.of(context);
    try {
      await user.setProfile(
          displayName: newDisplayName, profilePicture: newProfilePicture);
      setState(() {
        newDisplayName = null;
        newProfilePicture = null;
      });
    } catch (e) {
      showErrorBanner(messenger, "Error when saving profile: ${e.toString()}");
    }
  }

  @override
  Widget build(BuildContext context) {
    final (userName, displayName, profilePicture) = context.select(
      (UserCubit cubit) => (
        cubit.state.userName,
        cubit.state.displayName,
        cubit.state.profilePicture,
      ),
    );

    return Scaffold(
      appBar: AppBar(
        title: const Text('User Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            Column(
              children: [
                UserAvatar(
                  username: userName,
                  size: 100,
                  image: newProfilePicture ?? profilePicture,
                  onPressed: () async {
                    // Image picker
                    final ImagePicker picker = ImagePicker();
                    // Pick an image.
                    final XFile? image =
                        await picker.pickImage(source: ImageSource.gallery);
                    final bytes = await image?.readAsBytes();
                    setState(() {
                      newProfilePicture = bytes;
                    });
                  },
                ),
                const SizedBox(height: 15),
                Text(
                  userName,
                  style: const TextStyle(
                    color: colorDMB,
                    fontSize: 12,
                    fontVariations: variationRegular,
                    letterSpacing: -0.2,
                  ),
                ),
              ],
            ),
            Column(
              children: [
                const Text('Display name'),
                const SizedBox(height: 20),
                Form(
                  autovalidateMode: AutovalidateMode.always,
                  child: ConstrainedBox(
                    constraints: BoxConstraints.tight(const Size(300, 80)),
                    child: TextFormField(
                      autofocus: isSmallScreen(context) ? false : true,
                      decoration: inputDecoration.copyWith(
                        hintText: 'DISPLAY NAME',
                      ),
                      initialValue: displayName,
                      style: inputTextStyle,
                      onChanged: (value) {
                        setState(() {
                          newDisplayName = value;
                        });
                      },
                    ),
                  ),
                ),
              ],
            ),
            OutlinedButton(
              onPressed: _isChanged ? () => _save(context) : null,
              style: buttonStyle(context, _isChanged),
              child: const Text('Save'),
            )
          ],
        ),
      ),
    );
  }
}
