// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/messenger_view.dart';
import 'package:prototype/platform.dart';
import 'package:prototype/registration/server_choice.dart';
import 'package:prototype/settings/developer.dart';
import 'package:prototype/styles.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  var statusText = "";
  var signupRequired = false;

  @override
  void initState() {
    super.initState();
    initClient();
  }

  Future<void> initClient() async {
    setState(() {
      statusText = "Initializing core client...";
    });

    // iOS specific initialization
    if (Platform.isIOS) {
      // Initialize the method channel
      initMethodChannel();

      // Ask for notification permission
      var status = await Permission.notification.status;
      switch (status) {
        case PermissionStatus.denied:
          print("Notification permission denied, will ask the user");
          var requestStatus = await Permission.notification.request();
          print("The status is $requestStatus");
          break;
        default:
          print("Notification permission status: $status");
      }
    }

    await coreClient.loadUser().then((exists) {
      if (exists) {
        print("User loaded successfully");
        Navigator.pushReplacement(
          context,
          PageRouteBuilder(
            pageBuilder: (context, animation1, animation2) =>
                const MessengerView(),
            transitionDuration: Duration.zero,
            reverseTransitionDuration: Duration.zero,
          ),
        );
      } else {
        print("No user found, showing signup button");
        setState(() {
          signupRequired = true;
        });
      }
    });
  }

  Future<void> signup(BuildContext context) async {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (context) => const ServerChoice(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
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
              const GradientText(
                "Prototype.",
                gradient: LinearGradient(
                  colors: [
                    Color.fromARGB(255, 34, 163, 255),
                    Color.fromARGB(255, 72, 23, 250)
                  ],
                  transform: GradientRotation(1.1),
                ),
                style: TextStyle(
                  fontSize: 36,
                  fontVariations: variationMedium,
                  letterSpacing: -0.9,
                ),
              ),
              // Text button that opens the developer settings screen
              TextButton(
                onPressed: () {
                  Navigator.push(
                    context,
                    MaterialPageRoute(
                      builder: (context) => const DeveloperSettingsScreen(),
                    ),
                  );
                },
                style: textButtonStyle(context),
                child: const Text('Developer Settings'),
              ),
              signupRequired
                  ? Column(
                      crossAxisAlignment: isSmallScreen(context)
                          ? CrossAxisAlignment.stretch
                          : CrossAxisAlignment.center,
                      children: [
                        OutlinedButton(
                          onPressed: () => signup(context),
                          style: buttonStyle(context, true),
                          child: const Text('Sign up'),
                        )
                      ],
                    )
                  : Container(),
            ],
          ),
        ),
      ),
    );
  }
}

class GradientText extends StatelessWidget {
  const GradientText(
    this.text, {
    super.key,
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
