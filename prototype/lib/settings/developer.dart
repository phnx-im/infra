// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:prototype/core/api/user.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/homescreen.dart';
import 'package:prototype/main.dart';
import 'package:prototype/platform.dart';
import 'package:prototype/styles.dart';

class DeveloperSettingsScreen extends StatefulWidget {
  const DeveloperSettingsScreen({super.key});

  @override
  State<DeveloperSettingsScreen> createState() =>
      _DeveloperSettingsScreenState();
}

class _DeveloperSettingsScreenState extends State<DeveloperSettingsScreen> {
  String? deviceToken;

  @override
  void initState() {
    super.initState();
    getDeviceToken().then((token) {
      setState(() {
        deviceToken = token;
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: ListView(
        // space between tiles
        children: [
          ListTile(
            title: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  "Device token (iOS only):",
                  style: labelStyle,
                ),
                const SizedBox(height: 10),
                SelectableText(
                  deviceToken ?? "N/A",
                  style: labelStyle,
                ),
                const SizedBox(height: 10),
                if (deviceToken != null)
                  TextButton(
                      style: ButtonStyle(
                        foregroundColor:
                            WidgetStateProperty.all<Color>(colorDMB),
                        textStyle: WidgetStateProperty.all<TextStyle>(
                          TextStyle(
                            fontVariations: variationSemiBold,
                            fontFamily: fontFamily,
                            fontSize: isSmallScreen(context) ? 16 : 14,
                          ),
                        ),
                      ),
                      onPressed: () {
                        Clipboard.setData(
                            ClipboardData(text: deviceToken ?? ""));
                      },
                      child: const Text('Copy to clipboard')),
              ],
            ),
          ),
          const ListTile(
            title: SizedBox(
              height: 10,
            ),
          ),
          ListTile(
            title: OutlinedButton(
              style: buttonStyle(context, Platform.isIOS),
              onPressed: () async {
                final deviceToken = await getDeviceToken();
                if (deviceToken != null) {
                  final pushToken = PlatformPushToken.apple(deviceToken);
                  coreClient.user.updatePushToken(pushToken: pushToken);
                }
              },
              child: const Text('Re-register push token'),
            ),
          ),
          ListTile(
            title: OutlinedButton(
              style: buttonStyle(context, true),
              onPressed: () {
                showDialog(
                  context: context,
                  builder: (BuildContext context) {
                    return AlertDialog(
                      title: const Text('Confirmation'),
                      content: const Text(
                          'Are you sure you want to erase the database?'),
                      actions: [
                        TextButton(
                          style: textButtonStyle(context),
                          child: const Text('Cancel'),
                          onPressed: () {
                            Navigator.of(context).pop();
                          },
                        ),
                        TextButton(
                          style: textButtonStyle(context),
                          child: const Text('Erase'),
                          onPressed: () {
                            // Perform database erase operation
                            try {
                              coreClient.deleteDatabase().then((value) {
                                if (appNavigator.currentState != null) {
                                  // Remove all routes from the navigator stack and push the HomeScreen
                                  Navigator.pushAndRemoveUntil(
                                    appNavigator.currentState!.context,
                                    PageRouteBuilder(
                                      pageBuilder:
                                          (context, animation1, animation2) =>
                                              const HomeScreen(),
                                      transitionDuration: Duration.zero,
                                      reverseTransitionDuration: Duration.zero,
                                    ),
                                    (route) => false,
                                  );
                                }
                              });
                            } catch (e) {
                              showErrorBanner(
                                  context, "Could not delete databases: $e");
                              print(e);
                            }
                          },
                        ),
                      ],
                    );
                  },
                );
              },
              child: const Text('Erase Database'),
            ),
          ),
        ],
      ),
    );
  }
}
