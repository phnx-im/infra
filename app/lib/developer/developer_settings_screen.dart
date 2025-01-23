// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/main.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/util/platform.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

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

  bool _canReRegisterPushToken() =>
      isTouch() && context.read<LoadableUserCubit>().state.user != null;

  void _reRegisterPushToken(CoreClient coreClient) async {
    if (_canReRegisterPushToken()) {
      final deviceToken = await getDeviceToken();
      if (deviceToken != null) {
        if (Platform.isAndroid) {
          final pushToken = PlatformPushToken.google(deviceToken);
          coreClient.user.updatePushToken(pushToken);
        } else if (Platform.isIOS) {
          final pushToken = PlatformPushToken.apple(deviceToken);
          coreClient.user.updatePushToken(pushToken);
        }
      }
    }
  }

  void _confirmEraseDatabase() {
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
              onPressed: _eraseDatabase,
              child: const Text('Erase'),
            ),
          ],
        );
      },
    );
  }

  void _eraseDatabase() async {
    // Perform database erase operation
    final messengerState = ScaffoldMessenger.of(context);
    try {
      await context.read<CoreClient>().deleteDatabase();
    } catch (e) {
      showErrorBanner(messengerState, "Could not delete databases: $e");
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
        if (_canReRegisterPushToken())
          OutlinedButton(
            style: buttonStyle(context, _canReRegisterPushToken()),
            onPressed: () async {
              if (_canReRegisterPushToken()) {
                _reRegisterPushToken(context.read<CoreClient>());
              }
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
      onPressed: _confirmEraseDatabase,
      child: const Text('Erase Database'),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: const AppBarBackButton(),
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
