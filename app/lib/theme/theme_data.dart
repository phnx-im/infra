// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/ui/colors/palette.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/ui/theme/font.dart';
import 'package:prototype/ui/typography/font_size.dart';

ThemeData themeData(
  BuildContext context, {
  ColorScheme? colorScheme,
}) => ThemeData(
  colorScheme: colorScheme ?? lightColorScheme,
  appBarTheme: AppBarTheme(
    color: customColors(context).backgroundBase.primary,
    elevation: 0,
    iconTheme: IconThemeData(color: customColors(context).text.primary),
    toolbarHeight: isPointer() ? 100 : null,
    titleTextStyle: TextStyle(
      color: customColors(context).text.primary,
      fontSize: LabelFontSize.base.size,
      fontWeight: FontWeight.bold,
    ),
  ),
  scaffoldBackgroundColor: customColors(context).backgroundBase.primary,
  textTheme: customTextScheme,
  canvasColor: customColors(context).backgroundBase.primary,
  cardColor: customColors(context).backgroundBase.primary,
  dialogTheme: DialogThemeData(
    backgroundColor: customColors(context).backgroundBase.primary,
    surfaceTintColor: customColors(context).backgroundBase.primary,
  ),
  splashColor: Colors.transparent,
  highlightColor: Colors.transparent,
  hoverColor: Colors.transparent,
  outlinedButtonTheme: OutlinedButtonThemeData(
    style: buttonStyle(context, true),
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
      color: customColors(context).text.secondary,
      fontSize: LabelFontSize.small1.size,
    ),
    focusedBorder: _textInputBorder,
    enabledBorder: _textInputBorder,
    errorBorder: _textInputBorder,
    focusedErrorBorder: _textInputBorder,
    filled: true,
    fillColor: customColors(context).backgroundBase.secondary,
  ),
  switchTheme: SwitchThemeData(
    thumbColor: WidgetStateProperty.all(customColors(context).text.secondary),
    trackOutlineColor: WidgetStateProperty.all(
      customColors(context).separator.primary,
    ),
    trackColor: WidgetStateProperty.resolveWith(
      (states) =>
          states.contains(WidgetState.selected)
              ? customColors(context).backgroundBase.secondary
              : Colors.transparent,
    ),
  ),
);

final _textInputBorder = OutlineInputBorder(
  borderSide: const BorderSide(width: 0, style: BorderStyle.none),
  borderRadius: BorderRadius.circular(8),
);
