// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/registration/username_password.dart';
import 'package:prototype/styles.dart';

class ServerChoice extends StatefulWidget {
  const ServerChoice({super.key});

  @override
  State<ServerChoice> createState() => _ServerChoiceState();
}

const initialDomain = '';

class _ServerChoiceState extends State<ServerChoice> {
  String _domain = initialDomain;
  final bool _isProcessing = false;

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
            crossAxisAlignment: CrossAxisAlignment.center,
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: [
              const Text(
                'Choose a server where you want to create your account',
                style: labelStyle,
              ),
              Form(
                autovalidateMode: AutovalidateMode.always,
                child: Column(
                  children: [
                    const SizedBox(height: 10),
                    ConstrainedBox(
                      constraints: BoxConstraints.tight(const Size(300, 80)),
                      child: TextFormField(
                        autofocus: (Platform.isIOS || Platform.isAndroid)
                            ? false
                            : true,
                        decoration: inputDecoration.copyWith(
                          hintText: 'DOMAIN NAME',
                        ),
                        initialValue: initialDomain,
                        style: inputTextStyle,
                        onChanged: (String value) {
                          setState(() {
                            _domain = value;
                          });
                        },
                      ),
                    ),
                  ],
                ),
              ),
              Column(
                crossAxisAlignment: isSmallScreen(context)
                    ? CrossAxisAlignment.stretch
                    : CrossAxisAlignment.center,
                children: [
                  OutlinedButton(
                    onPressed: () => {
                      if (!_isProcessing)
                        Navigator.push(
                          context,
                          PageRouteBuilder(
                            pageBuilder: (context, animation1, animation2) =>
                                UsernamePasswordChoice(domain: _domain),
                            transitionDuration:
                                const Duration(milliseconds: 150),
                            transitionsBuilder: (_, a, __, c) =>
                                FadeTransition(opacity: a, child: c),
                          ),
                        )
                    },
                    style: buttonStyle(context, !_isProcessing),
                    child: const Text('Next'),
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
