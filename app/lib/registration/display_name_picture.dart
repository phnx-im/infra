// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/app.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/main.dart';
import 'package:prototype/messenger_view.dart';
import 'package:prototype/styles.dart';

class DisplayNameAvatarChoice extends StatefulWidget {
  final String domain;
  final String username;
  final String password;
  const DisplayNameAvatarChoice(
      {super.key,
      required this.domain,
      required this.username,
      required this.password});

  @override
  State<DisplayNameAvatarChoice> createState() =>
      _DisplayNameAvatarChoiceState();
}

class _DisplayNameAvatarChoiceState extends State<DisplayNameAvatarChoice> {
  Uint8List? _avatar;
  String? _displayName;
  bool _isProcessing = false;

  @override
  void initState() {
    super.initState();
    _displayName = widget.username;
  }

  @override
  void deactivate() {
    super.deactivate();
    if (appNavigator.currentState != null) {
      ScaffoldMessenger.of(appNavigator.currentState!.context)
          .hideCurrentMaterialBanner();
    }
  }

  Future<void> signup() async {
    final coreClient = context.coreClient;

    final domain = widget.domain;
    final username = widget.username;
    final password = widget.password;
    final fqun = "$username@$domain";
    final url = "https://$domain:443";

    print("Registering user $username ...");

    setState(() {
      _isProcessing = true;
    });

    try {
      await coreClient.createUser(fqun, password, url);
    } catch (e) {
      print("Error when registering user: $e");
      if (mounted) {
        showErrorBanner(
            context, "Error when registering user: ${e.toString()}");
        setState(() {
          _isProcessing = false;
        });
      }
      return;
    }

    // Set the user's display name and profile picture
    try {
      await coreClient.setOwnProfile(_displayName ?? "", _avatar);
    } catch (e) {
      print("Error when setting profile: $e");
      if (mounted) {
        showErrorBanner(context, "Error when setting profile: ${e.toString()}");
        setState(() {
          _isProcessing = false;
        });
      }
      return;
    }

    // Replace all previous routes with the MessengerView
    if (mounted) {
      Navigator.pushAndRemoveUntil(
        context,
        PageRouteBuilder(
          pageBuilder: (context, animation1, animation2) =>
              const MessengerView(),
          transitionDuration: const Duration(milliseconds: 150),
          transitionsBuilder: (_, a, __, c) =>
              FadeTransition(opacity: a, child: c),
        ),
        (route) => false,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      resizeToAvoidBottomInset: false,
      appBar: AppBar(
        title: const Text('Sign up'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: Padding(
        padding: const EdgeInsets.all(20.0),
        child: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              UserAvatar(
                username: widget.username,
                size: 100,
                image: _avatar,
                onPressed: () async {
                  // Image picker
                  final ImagePicker picker = ImagePicker();
                  // Pick an image.
                  final XFile? image =
                      await picker.pickImage(source: ImageSource.gallery);
                  image?.readAsBytes().then((value) {
                    setState(() {
                      _avatar = value;
                    });
                  });
                },
              ),
              Column(
                children: [
                  const Text('Choose a picture and a display name'),
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
                        initialValue: widget.username,
                        style: inputTextStyle,
                        onChanged: (value) {
                          setState(() {
                            _displayName = value;
                          });
                        },
                      ),
                    ),
                  ),
                ],
              ),
              Column(
                crossAxisAlignment: isSmallScreen(context)
                    ? CrossAxisAlignment.stretch
                    : CrossAxisAlignment.center,
                children: [
                  if (!_isProcessing)
                    OutlinedButton(
                      onPressed: () => {if (!_isProcessing) signup()},
                      style: buttonStyle(context, !_isProcessing),
                      child: const Text('Sign up'),
                    ),
                  if (_isProcessing)
                    const Align(
                      child: CircularProgressIndicator(
                        value: null,
                        valueColor: AlwaysStoppedAnimation<Color>(colorDMB),
                        backgroundColor: Colors.transparent,
                      ),
                    ),
                ],
              )
            ],
          ),
        ),
      ),
    );
  }
}
