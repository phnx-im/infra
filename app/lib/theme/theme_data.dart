// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/theme/theme.dart';

const _defaultLetterSpacing = -0.2;

ThemeData themeData(BuildContext context) => ThemeData(
      appBarTheme: AppBarTheme(
        color: Colors.white,
        elevation: 0,
        iconTheme: const IconThemeData(color: Colors.black),
        surfaceTintColor: Colors.black,
        titleTextStyle: TextStyle(
          fontFamily: fontFamily,
          color: Colors.black,
          letterSpacing: _defaultLetterSpacing,
        ).merge(VariableFontWeight.bold),
      ),
      fontFamily: fontFamily,
      textTheme: TextTheme(
        displayLarge: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w400),
        displayMedium: TextStyle(letterSpacing: _defaultLetterSpacing),
        displaySmall: TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineLarge: TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineMedium: TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineSmall: TextStyle(letterSpacing: _defaultLetterSpacing),
        titleLarge: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        titleMedium: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        titleSmall: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        bodyLarge: TextStyle(letterSpacing: _defaultLetterSpacing),
        bodyMedium: TextStyle(letterSpacing: _defaultLetterSpacing),
        bodySmall: TextStyle(letterSpacing: _defaultLetterSpacing),
        labelLarge: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        labelMedium: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        labelSmall: TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
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
