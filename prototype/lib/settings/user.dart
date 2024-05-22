// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/main.dart';
import 'package:prototype/styles.dart';

class UserSettingsScreen extends StatefulWidget {
  const UserSettingsScreen({super.key});

  @override
  State<UserSettingsScreen> createState() => _UserSettingsScreenState();
}

class _UserSettingsScreenState extends State<UserSettingsScreen> {
  Uint8List? avatar = coreClient.ownProfile.profilePictureOption;
  String? displayName = coreClient.ownProfile.displayName;
  bool imageChanged = false;
  bool displayNameChanged = false;

  @override
  void initState() {
    super.initState();
  }

  bool changed() {
    return imageChanged || displayNameChanged;
  }

  void save(BuildContext context) {
    try {
      coreClient.setOwnProfile(displayName ?? "", avatar).then((value) {
        Navigator.of(context).pop();
      });
      setState(() {
        imageChanged = false;
        displayNameChanged = false;
      });
    } catch (e) {
      showErrorBanner(context, "Error when saving profile: ${e.toString()}");
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('User Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            Column(
              children: [
                UserAvatar(
                  username: coreClient.username,
                  size: 100,
                  image: avatar,
                  onPressed: () async {
                    // Image picker
                    final ImagePicker picker = ImagePicker();
                    // Pick an image.
                    final XFile? image =
                        await picker.pickImage(source: ImageSource.gallery);
                    image?.readAsBytes().then((value) {
                      setState(() {
                        avatar = value;
                        imageChanged = true;
                      });
                    });
                  },
                ),
                const SizedBox(height: 15),
                Text(
                  coreClient.username,
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
                          displayName = value;
                          displayNameChanged = true;
                        });
                      },
                    ),
                  ),
                ),
              ],
            ),
            OutlinedButton(
              onPressed: () {
                if (changed()) {
                  save(context);
                }
              },
              style: buttonStyle(context, changed()),
              child: const Text('Save'),
            )
          ],
        ),
      ),
    );
  }
}
