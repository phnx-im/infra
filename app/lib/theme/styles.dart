// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
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

// === Fonts ===

const fontFamily = "InterEmbedded";

const variationRegular = [FontVariation("wght", 420)];
const variationMedium = [FontVariation("wght", 500)];
const variationSemiBold = [FontVariation("wght", 600)];
const variationBold = [FontVariation("wght", 700)];

// === Colors ===

// DMB

const colorDMB = Color(0xFF616E7E);
const colorDMBLight = Color(0xFFC3C8CE);
const colorDMBSuperLight = Color(0xFFEFF1F2);

// Grey

const colorGrey = Color(0xFFC4C4C4);
const colorGreyLight = Color(0xFFDEDEDE);
const colorGreySuperLight = Color(0xFFFAFAFA);
const colorGreyDark = Color(0xFF8A8A8A);

const swatchColor = Color(0xFFC0C6CE);

const activeButtonColor = colorDMB;
const inactiveButtonColor = colorDMBSuperLight;

// === Text ===

const labelStyle = TextStyle(
  fontFamily: fontFamily,
  fontSize: 14,
  fontVariations: variationRegular,
  letterSpacing: -0.02,
);

const boldLabelStyle = TextStyle(
  fontFamily: fontFamily,
  fontSize: 14,
  fontVariations: variationSemiBold,
  letterSpacing: -0.02,
);

// === Inputs ===

const inputTextStyle = TextStyle(
  fontFamily: fontFamily,
  fontSize: 14,
  fontVariations: variationRegular,
);

final inputDecoration = InputDecoration(
  border: InputBorder.none,
  hintStyle: const TextStyle(
    color: colorDMBLight,
    fontSize: 11,
    fontWeight: FontWeight.w100,
    fontFamily: fontFamily,
  ),
  focusedBorder: textInputBorder,
  enabledBorder: textInputBorder,
  errorBorder: textInputBorder,
  focusedErrorBorder: textInputBorder,
  filled: true,
  fillColor: colorDMBSuperLight,
);

InputDecoration messageComposerInputDecoration(BuildContext context) =>
    InputDecoration(
      border: InputBorder.none,
      hintStyle: TextStyle(
        color: colorGrey,
        fontSize: isLargeScreen(context) ? 12 : 14,
        fontWeight: FontWeight.w400,
        fontFamily: fontFamily,
      ),
      focusedBorder: textInputBorder,
      enabledBorder: textInputBorder,
      errorBorder: textInputBorder,
      focusedErrorBorder: textInputBorder,
      filled: true,
      fillColor: Colors.white,
    );

TextStyle messageTextStyle(BuildContext context, bool inverted) => TextStyle(
      color: inverted ? Colors.white : Colors.black,
      fontFamily: fontFamily,
      fontVariations:
          isLargeScreen(context) ? variationRegular : variationMedium,
      letterSpacing: -0.05,
      fontSize: isLargeScreen(context) ? 14 : 15,
      // NOTE: When specifying line height, the text is rendered inconsistently on
      // Linux and macOS (and therefore also on Android and iOS). For now, we use the default one.
      // height: isLargeScreen(context) ? 1.5 : 1.3,
    );

final textInputBorder = OutlineInputBorder(
  borderSide: const BorderSide(
    color: Colors.white,
    width: 0,
    style: BorderStyle.none,
  ),
  borderRadius: BorderRadius.circular(7),
);

// === Buttons ===

ButtonStyle textButtonStyle(BuildContext context) {
  return ButtonStyle(
    foregroundColor: WidgetStateProperty.all(colorDMB),
    overlayColor: WidgetStateProperty.all(Colors.transparent),
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    splashFactory: NoSplash.splashFactory,
    padding: WidgetStateProperty.all(const EdgeInsets.all(20)),
    textStyle: WidgetStateProperty.all<TextStyle>(
      TextStyle(
        fontVariations: variationSemiBold,
        fontFamily: fontFamily,
        fontSize: isSmallScreen(context) ? 16 : 14,
      ),
    ),
  );
}

ButtonStyle dynamicTextButtonStyle(
    BuildContext context, bool isActive, bool isMain) {
  return ButtonStyle(
    foregroundColor: isActive
        ? WidgetStateProperty.all(colorDMB)
        : WidgetStateProperty.all(colorDMBLight),
    overlayColor: WidgetStateProperty.all(Colors.transparent),
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    splashFactory: NoSplash.splashFactory,
    padding: WidgetStateProperty.all(const EdgeInsets.all(20)),
    textStyle: WidgetStateProperty.all<TextStyle>(
      TextStyle(
        fontVariations: isMain ? variationSemiBold : variationMedium,
        fontFamily: fontFamily,
        fontSize: isSmallScreen(context) ? 16 : 14,
      ),
    ),
  );
}

ButtonStyle buttonStyle(BuildContext context, bool isActive) {
  return ButtonStyle(
    foregroundColor: WidgetStateProperty.all<Color>(
        isActive ? Colors.white : activeButtonColor),
    backgroundColor: WidgetStateProperty.all<Color>(
        isActive ? activeButtonColor : inactiveButtonColor),
    overlayColor: WidgetStateProperty.all<Color>(
        isActive ? activeButtonColor : inactiveButtonColor),
    mouseCursor: WidgetStateProperty.all<MouseCursor>(
        isActive ? SystemMouseCursors.click : SystemMouseCursors.basic),
    elevation: WidgetStateProperty.all<double>(0),
    shadowColor: WidgetStateProperty.all<Color>(Colors.transparent),
    padding: WidgetStateProperty.all<EdgeInsetsGeometry>(
        const EdgeInsets.symmetric(vertical: 25, horizontal: 50)),
    splashFactory: NoSplash.splashFactory,
    surfaceTintColor: WidgetStateProperty.all<Color>(Colors.transparent),
    side: WidgetStateProperty.all<BorderSide>(
        const BorderSide(color: Colors.transparent, width: 0)),
    shape: WidgetStateProperty.all<OutlinedBorder>(
      RoundedRectangleBorder(
        side: const BorderSide(
          color: Colors.transparent,
          width: 0,
          style: BorderStyle.none,
        ),
        borderRadius: isSmallScreen(context)
            ? BorderRadius.circular(12)
            : BorderRadius.circular(7),
      ),
    ),
    textStyle: WidgetStateProperty.all<TextStyle>(
      TextStyle(
        fontVariations: variationSemiBold,
        fontFamily: fontFamily,
        fontSize: isSmallScreen(context) ? 16 : 14,
      ),
    ),
  );
}

// === Left pane ===

const convPaneBackgroundColor = colorDMBSuperLight;
const convPaneFocusColor = colorGreyLight;
const convPaneBlurColor = Color(0x00FFFFFF);

// === Conversation list ===

const convListItemTextColor = Color(0xFF000000);
const convListItemSelectedColor = Color(0xFF000000);
