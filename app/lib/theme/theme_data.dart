// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/palette.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/ui/theme/font.dart';
import 'package:air/ui/typography/font_size.dart';

ThemeData darkTheme = themeData(Brightness.dark, darkCustomColorScheme);
ThemeData lightTheme = themeData(Brightness.light, lightCustomColorScheme);

ThemeData themeData(
  Brightness brightness,
  CustomColorScheme colorScheme,
) => ThemeData(
  colorScheme: ColorScheme(
    brightness: brightness,
    primary: colorScheme.text.primary,
    onPrimary: colorScheme.backgroundBase.primary,
    secondary: colorScheme.text.secondary,
    onSecondary: colorScheme.backgroundBase.primary,
    surface: colorScheme.backgroundBase.primary,
    onSurface: colorScheme.text.primary,
    error: colorScheme.function.danger,
    onError: colorScheme.text.primary,
  ),
  appBarTheme: AppBarTheme(
    backgroundColor: colorScheme.backgroundBase.primary,
    elevation: 0,
    iconTheme: IconThemeData(color: colorScheme.text.primary),
    toolbarHeight: isPointer() ? 100 : null,
    titleTextStyle: TextStyle(
      color: colorScheme.text.primary,
      fontSize: LabelFontSize.base.size,
      fontWeight: FontWeight.bold,
    ),
  ),
  scaffoldBackgroundColor: colorScheme.backgroundBase.primary,
  textTheme: customTextScheme,
  canvasColor: colorScheme.backgroundBase.primary,
  cardColor: colorScheme.backgroundBase.primary,
  dialogTheme: DialogThemeData(
    backgroundColor: colorScheme.backgroundBase.primary,
    surfaceTintColor: colorScheme.backgroundBase.primary,
  ),
  splashColor: Colors.transparent,
  highlightColor: Colors.transparent,
  hoverColor: Colors.transparent,
  outlinedButtonTheme: OutlinedButtonThemeData(
    style: buttonStyleFromColorScheme(colorScheme, true),
  ),
  iconButtonTheme: IconButtonThemeData(
    style: ButtonStyle(
      splashFactory: NoSplash.splashFactory,
      surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
      overlayColor: WidgetStateProperty.all(Colors.transparent),
    ),
  ),
  textSelectionTheme: TextSelectionThemeData(cursorColor: AppColors.blue[300]),
  inputDecorationTheme: InputDecorationTheme(
    border: InputBorder.none,
    hintStyle: TextStyle(
      color: colorScheme.text.secondary,
      fontSize: LabelFontSize.small1.size,
    ),
    focusedBorder: _textInputBorder,
    enabledBorder: _textInputBorder,
    errorBorder: _textInputBorder,
    focusedErrorBorder: _textInputBorder,
    filled: true,
    fillColor: colorScheme.backgroundBase.secondary,
  ),
  switchTheme: SwitchThemeData(
    thumbColor: WidgetStateProperty.all(colorScheme.text.secondary),
    trackOutlineColor: WidgetStateProperty.all(colorScheme.separator.primary),
    trackColor: WidgetStateProperty.resolveWith(
      (states) =>
          states.contains(WidgetState.selected)
              ? colorScheme.backgroundBase.secondary
              : Colors.transparent,
    ),
  ),
);

final _textInputBorder = OutlineInputBorder(
  borderSide: const BorderSide(width: 0, style: BorderStyle.none),
  borderRadius: BorderRadius.circular(8),
);
