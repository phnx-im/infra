// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/ui/typography/font_size.dart';
import 'dart:io' show Platform;

// === Devices ===

bool isSmallScreen(BuildContext context) {
  return MediaQuery.of(context).size.width <= 600;
}

bool isLargeScreen(BuildContext context) {
  return MediaQuery.of(context).size.width > 600;
}

bool isTouch() {
  return Platform.isIOS || Platform.isAndroid;
}

bool isPointer() {
  return Platform.isLinux || Platform.isMacOS || Platform.isWindows;
}

// === Colors ===

// Grey

Color activeButtonColor(CustomColorScheme colorScheme) =>
    colorScheme.backgroundBase.quaternary;
Color inactiveButtonColor(CustomColorScheme colorScheme) =>
    colorScheme.backgroundBase.secondary;

// === Buttons ===

ButtonStyle textButtonStyle(BuildContext context) {
  return ButtonStyle(
    foregroundColor: WidgetStateProperty.all(
      customColors(context).text.primary,
    ),
    overlayColor: WidgetStateProperty.all(Colors.transparent),
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    splashFactory: NoSplash.splashFactory,
    padding: WidgetStateProperty.all(const EdgeInsets.all(20)),
    textStyle: WidgetStateProperty.all<TextStyle>(
      Theme.of(context).textTheme.labelLarge!.copyWith(
        fontSize: LabelFontSize.base.size,
        fontWeight: FontWeight.bold,
      ),
    ),
  );
}

ButtonStyle dynamicTextButtonStyle(
  BuildContext context,
  bool isActive,
  bool isMain,
) {
  return ButtonStyle(
    foregroundColor:
        isActive
            ? WidgetStateProperty.all(customColors(context).text.secondary)
            : WidgetStateProperty.all(customColors(context).text.quaternary),
    overlayColor: WidgetStateProperty.all(Colors.transparent),
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    splashFactory: NoSplash.splashFactory,
    padding: WidgetStateProperty.all(const EdgeInsets.all(20)),
    textStyle: WidgetStateProperty.all<TextStyle>(
      Theme.of(context).textTheme.labelLarge!.copyWith(
        fontSize: LabelFontSize.base.size,
        fontWeight: isMain ? FontWeight.bold : FontWeight.normal,
      ),
    ),
  );
}

ButtonStyle buttonStyle(CustomColorScheme colorScheme, bool isActive) {
  return ButtonStyle(
    foregroundColor: WidgetStateProperty.all<Color>(
      isActive ? colorScheme.text.primary : colorScheme.text.quaternary,
    ),
    backgroundColor: WidgetStateProperty.all<Color>(
      isActive
          ? activeButtonColor(colorScheme)
          : inactiveButtonColor(colorScheme),
    ),
    overlayColor: WidgetStateProperty.all<Color>(
      isActive
          ? activeButtonColor(colorScheme)
          : inactiveButtonColor(colorScheme),
    ),
    mouseCursor: WidgetStateProperty.all<MouseCursor>(
      isActive ? SystemMouseCursors.click : SystemMouseCursors.basic,
    ),
    elevation: WidgetStateProperty.all<double>(0),
    shadowColor: WidgetStateProperty.all<Color>(Colors.transparent),
    padding: WidgetStateProperty.all<EdgeInsetsGeometry>(
      const EdgeInsets.symmetric(vertical: 25, horizontal: 50),
    ),
    splashFactory: NoSplash.splashFactory,
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    side: WidgetStateProperty.all<BorderSide>(
      const BorderSide(color: Colors.transparent, width: 0),
    ),
    shape: WidgetStateProperty.all<OutlinedBorder>(
      RoundedRectangleBorder(
        side: const BorderSide(
          color: Colors.transparent,
          width: 0,
          style: BorderStyle.none,
        ),
        borderRadius: BorderRadius.circular(12),
      ),
    ),
    textStyle: WidgetStateProperty.all<TextStyle>(
      TextStyle(fontSize: LabelFontSize.base.size, fontWeight: FontWeight.bold),
    ),
  );
}
