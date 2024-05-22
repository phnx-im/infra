// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/homescreen.dart';
import 'package:prototype/main.dart';
import 'package:prototype/styles.dart';

class DeveloperSettingsScreen extends StatelessWidget {
  const DeveloperSettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer Settings'),
        toolbarHeight: isPointer() ? 100 : null,
        leading: appBarBackButton(context),
      ),
      body: ListView(
        children: [
          ListTile(
            title: TextButton(
              style: textButtonStyle(context),
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
                              coreClient.deleteDatabases().then((value) {
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
