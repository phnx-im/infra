// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/theme/theme.dart';

ThemeData themeData(BuildContext context) => ThemeData(
      appBarTheme: AppBarTheme(
        color: Colors.white,
        elevation: 0,
        iconTheme: const IconThemeData(color: Colors.black),
        surfaceTintColor: Colors.black,
        titleTextStyle: boldLabelStyle.copyWith(color: Colors.black),
      ),
      fontFamily: fontFamily,
      textTheme: TextTheme(
        displayLarge: TextStyle(letterSpacing: -0.2),
        displayMedium: TextStyle(letterSpacing: -0.2),
        displaySmall: TextStyle(letterSpacing: -0.2),
        headlineLarge: TextStyle(letterSpacing: -0.2),
        headlineMedium: TextStyle(letterSpacing: -0.2),
        headlineSmall: TextStyle(letterSpacing: -0.2),
        titleLarge: TextStyle(letterSpacing: -0.2),
        titleMedium: TextStyle(letterSpacing: -0.2),
        titleSmall: TextStyle(letterSpacing: -0.2),
        bodyLarge: TextStyle(letterSpacing: -0.2),
        bodyMedium: TextStyle(letterSpacing: -0.2),
        bodySmall: TextStyle(letterSpacing: -0.2),
        labelLarge: TextStyle(letterSpacing: -0.2),
        labelMedium: TextStyle(letterSpacing: -0.2),
        labelSmall: TextStyle(letterSpacing: -0.2),
      ),
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
          surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
          overlayColor: WidgetStateProperty.all(Colors.transparent),
        ),
      ),
      textSelectionTheme:
          const TextSelectionThemeData(cursorColor: Colors.blue),
    );
