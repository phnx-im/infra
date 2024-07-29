// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/homescreen.dart';
import 'package:prototype/styles.dart';

void main() async {
  // Initialize the FRB
  await coreClient.init();

  runApp(const MyApp());
}

final GlobalKey<NavigatorState> appNavigator = GlobalKey<NavigatorState>();

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Prototype',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        appBarTheme: AppBarTheme(
          color: Colors.white,
          elevation: 0,
          iconTheme: const IconThemeData(color: Colors.black),
          surfaceTintColor: Colors.black,
          titleTextStyle: boldLabelStyle.copyWith(color: Colors.black),
        ),
        fontFamily: fontFamily,
        textTheme: const TextTheme(),
        canvasColor: Colors.white,
        cardColor: Colors.white,
        colorScheme: ColorScheme.fromSwatch(
          accentColor: swatchColor,
          backgroundColor: Colors.white,
          brightness: Brightness.light,
        ),
        dialogBackgroundColor: Colors.white,
        dialogTheme: const DialogTheme(
          backgroundColor: Colors.white,
          surfaceTintColor: Colors.white,
        ),
        primaryColor: swatchColor,
        splashColor: Colors.transparent,
        highlightColor: Colors.transparent,
        hoverColor: Colors.transparent,
        outlinedButtonTheme:
            OutlinedButtonThemeData(style: buttonStyle(context, true)),
        iconButtonTheme: IconButtonThemeData(
          style: ButtonStyle(
            splashFactory: NoSplash.splashFactory,
            surfaceTintColor:
                WidgetStateProperty.all<Color>(Colors.transparent),
            overlayColor: WidgetStateProperty.all(Colors.transparent),
          ),
        ),
        textSelectionTheme:
            const TextSelectionThemeData(cursorColor: Colors.blue),
      ),
      navigatorKey: appNavigator,
      home: const HomeScreen(),
    );
  }
}

void showErrorBanner(BuildContext context, String errorDescription) {
  ScaffoldMessenger.of(context).showMaterialBanner(
    MaterialBanner(
      backgroundColor: Colors.red,
      leading: const Icon(Icons.error),
      padding: const EdgeInsets.all(20),
      content: Text(errorDescription),
      actions: [
        TextButton(
          child: const Text(
            'OK',
            style: TextStyle(color: Colors.white),
          ),
          onPressed: () {
            ScaffoldMessenger.of(context).hideCurrentMaterialBanner();
          },
        ),
      ],
    ),
  );
}
