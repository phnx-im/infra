// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/registration/display_name_picture.dart';
import 'package:prototype/styles.dart';

class UsernamePasswordChoice extends StatefulWidget {
  final String domain;

  const UsernamePasswordChoice({super.key, required this.domain});

  @override
  State<UsernamePasswordChoice> createState() => _UsernamePasswordChoiceState();
}

class _UsernamePasswordChoiceState extends State<UsernamePasswordChoice> {
  String _domain = '';
  String _username = '';
  String _password = '';
  bool _isUsernameValid = false;
  bool _isPasswordValid = false;

  @override
  void initState() {
    super.initState();
    _domain = widget.domain;
  }

  String? usernameValidator(String? value) {
    // alphanumeric
    final validCharacters = RegExp(r'^[a-zA-Z0-9@.]+$');
    var containsInvalidChars =
        value!.isNotEmpty && !validCharacters.hasMatch(value);
    var isTooLong = value.length >= 64;
    var isTooShort = value.isEmpty;
    var hasRightLength = !isTooShort && !isTooLong;
    _isUsernameValid = hasRightLength && !containsInvalidChars;
    if (_isUsernameValid) {
      _username = value;
      return null;
    } else {
      if (containsInvalidChars) {
        return 'Please use alphanumeric characters only';
      } else if (isTooLong) {
        return 'Maximum length is 64 characters';
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      resizeToAvoidBottomInset: false,
      appBar: AppBar(
        title: const Text('Sign up', style: TextStyle(fontFamily: fontFamily)),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: Padding(
        padding: const EdgeInsets.all(20.0),
        child: Center(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: [
              const Text(
                'Choose a username and password',
                style: labelStyle,
              ),
              Form(
                autovalidateMode: AutovalidateMode.always,
                child: Column(
                  children: [
                    const SizedBox(height: 5),
                    ConstrainedBox(
                      constraints: BoxConstraints.tight(const Size(300, 80)),
                      child: TextFormField(
                        autofocus: (Platform.isIOS || Platform.isAndroid)
                            ? false
                            : true,
                        decoration: inputDecoration.copyWith(
                          hintText: 'USERNAME',
                        ),
                        style: inputTextStyle,
                        validator: usernameValidator,
                        onChanged: (String value) {
                          final validCharacters = RegExp(r'^[a-zA-Z0-9@.]+$');
                          var containsInvalidChars = value.isNotEmpty &&
                              !validCharacters.hasMatch(value);
                          var hasRightLength =
                              value.isNotEmpty && value.length <= 64;
                          setState(() {
                            _isUsernameValid =
                                hasRightLength && !containsInvalidChars;
                            _username = value;
                          });
                        },
                      ),
                    ),
                    const SizedBox(height: 5),
                    ConstrainedBox(
                      constraints: BoxConstraints.tight(const Size(300, 80)),
                      child: TextFormField(
                        decoration: inputDecoration.copyWith(
                          hintText: 'PASSWORD',
                        ),
                        style: inputTextStyle,
                        obscureText: true,
                        onChanged: (String value) {
                          setState(() {
                            _isPasswordValid = value.isNotEmpty;
                            _password = value;
                          });
                        },
                      ),
                    )
                  ],
                ),
              ),
              Column(
                crossAxisAlignment: isSmallScreen(context)
                    ? CrossAxisAlignment.stretch
                    : CrossAxisAlignment.center,
                children: [
                  OutlinedButton(
                    style: buttonStyle(
                        context, _isUsernameValid && _isPasswordValid),
                    child: const Text('Next'),
                    onPressed: () => {
                      if (_isUsernameValid && _isPasswordValid)
                        Navigator.push(
                          context,
                          PageRouteBuilder(
                            pageBuilder: (context, animation1, animation2) =>
                                DisplayNameAvatarChoice(
                                    domain: _domain,
                                    username: _username,
                                    password: _password),
                            transitionDuration:
                                const Duration(milliseconds: 150),
                            transitionsBuilder: (_, a, __, c) =>
                                FadeTransition(opacity: a, child: c),
                          ),
                        )
                    },
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
