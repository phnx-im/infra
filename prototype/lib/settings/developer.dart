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

  bool canReRegisterPushToken() {
    return isTouch() && coreClient.maybeUser != null;
  }

  void reRegisterPushToken() async {
    if (canReRegisterPushToken()) {
      final deviceToken = await getDeviceToken();
      if (deviceToken != null) {
        if (Platform.isAndroid) {
          final pushToken = PlatformPushToken.google(deviceToken);
          coreClient.user.updatePushToken(pushToken: pushToken);
        } else if (Platform.isIOS) {
          final pushToken = PlatformPushToken.apple(deviceToken);
          coreClient.user.updatePushToken(pushToken: pushToken);
        }
      }
    }
  }

  void confirmEraseDatabase() {
    showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text('Confirmation'),
          content: const Text('Are you sure you want to erase the database?'),
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
              onPressed: eraseDatabase,
              child: const Text('Erase'),
            ),
          ],
        );
      },
    );
  }

  void eraseDatabase() {
    // Perform database erase operation
    try {
      coreClient.deleteDatabase().then((value) {
        if (appNavigator.currentState != null) {
          // Remove all routes from the navigator stack and push the HomeScreen
          var appContext = appNavigator.currentState!.context;
          if (appContext.mounted) {
            Navigator.pushAndRemoveUntil(
              appContext,
              PageRouteBuilder(
                pageBuilder: (context, animation1, animation2) =>
                    const HomeScreen(),
                transitionDuration: Duration.zero,
                reverseTransitionDuration: Duration.zero,
              ),
              (route) => false,
            );
          }
        }
      });
    } catch (e) {
      showErrorBanner(context, "Could not delete databases: $e");
      print(e);
    }
  }

  Widget deviceTokenElement() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Text(
          "Device token (mobile only):",
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
              foregroundColor: WidgetStateProperty.all<Color>(colorDMB),
              textStyle: WidgetStateProperty.all<TextStyle>(
                TextStyle(
                  fontVariations: variationSemiBold,
                  fontFamily: fontFamily,
                  fontSize: isSmallScreen(context) ? 16 : 14,
                ),
              ),
            ),
            onPressed: () {
              Clipboard.setData(ClipboardData(text: deviceToken ?? ""));
            },
            child: const Text('Copy to clipboard'),
          ),
        const SizedBox(height: 10),
        if (canReRegisterPushToken())
          OutlinedButton(
            style: buttonStyle(context, canReRegisterPushToken()),
            onPressed: () async {
              if (canReRegisterPushToken()) reRegisterPushToken();
            },
            child: const Text('Re-register push token'),
          ),
      ]
          .map((widget) => Padding(
                padding: const EdgeInsets.symmetric(vertical: 5),
                child: widget,
              ))
          .toList(),
    );
  }

  Widget eraseDatabaseElement() {
    return OutlinedButton(
      style: buttonStyle(context, true),
      onPressed: confirmEraseDatabase,
      child: const Text('Erase Database'),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: SingleChildScrollView(
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (isTouch()) deviceTokenElement(),
              eraseDatabaseElement(),
            ]
                .map((widget) => Padding(
                      padding: const EdgeInsets.symmetric(vertical: 20),
                      child: widget,
                    ))
                .toList(),
          ),
        ),
      ),
    );
  }
}
