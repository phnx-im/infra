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
        titleTextStyle: const TextStyle(
          fontFamily: fontFamily,
          color: Colors.black,
          letterSpacing: _defaultLetterSpacing,
        ).merge(VariableFontWeight.bold),
      ),
      fontFamily: fontFamily,
      textTheme: TextTheme(
        displayLarge: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w400),
        displayMedium: const TextStyle(letterSpacing: _defaultLetterSpacing),
        displaySmall: const TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineLarge: const TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineMedium: const TextStyle(letterSpacing: _defaultLetterSpacing),
        headlineSmall: const TextStyle(letterSpacing: _defaultLetterSpacing),
        titleLarge: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        titleMedium: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        titleSmall: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        bodyLarge: const TextStyle(letterSpacing: _defaultLetterSpacing),
        bodyMedium: const TextStyle(letterSpacing: _defaultLetterSpacing),
        bodySmall: const TextStyle(letterSpacing: _defaultLetterSpacing),
        labelLarge: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        labelMedium: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
        labelSmall: const TextStyle(letterSpacing: _defaultLetterSpacing)
            .merge(VariableFontWeight.w500),
      ),
      canvasColor: Colors.white,
      cardColor: Colors.white,
      colorScheme: ColorScheme.fromSwatch(
        accentColor: swatchColor,
        backgroundColor: Colors.white,
        brightness: Brightness.light,
      ),
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
      inputDecorationTheme: InputDecorationTheme(
        border: InputBorder.none,
        hintStyle: const TextStyle(
          color: colorDMBLight,
          fontSize: 11,
          fontFamily: fontFamily,
        ).merge(VariableFontWeight.normal),
        focusedBorder: _textInputBorder,
        enabledBorder: _textInputBorder,
        errorBorder: _textInputBorder,
        focusedErrorBorder: _textInputBorder,
        filled: true,
        fillColor: colorDMBSuperLight,
      ),
    );

final _textInputBorder = OutlineInputBorder(
  borderSide: const BorderSide(
    width: 0,
    style: BorderStyle.none,
  ),
  borderRadius: BorderRadius.circular(7),
);
