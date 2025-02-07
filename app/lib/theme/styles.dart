// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'dart:io' show Platform;

import 'variable_font_weight.dart';

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

// === Inputs ===

final inputTextStyle = TextStyle(
  fontFamily: fontFamily,
  fontSize: 14,
).merge(VariableFontWeight.w400);

final inputDecoration = InputDecoration(
  border: InputBorder.none,
  hintStyle: const TextStyle(
    color: colorDMBLight,
    fontSize: 11,
    fontFamily: fontFamily,
  ).merge(VariableFontWeight.w100),
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
      hintStyle: DefaultTextStyle.of(context)
          .style
          .copyWith(
            color: colorGrey,
            fontSize: isLargeScreen(context) ? 12 : 14,
            fontFamily: fontFamily,
          )
          .merge(VariableFontWeight.w400),
      focusedBorder: textInputBorder,
      enabledBorder: textInputBorder,
      errorBorder: textInputBorder,
      focusedErrorBorder: textInputBorder,
      filled: true,
      fillColor: Colors.white,
    );

TextStyle messageTextStyle(BuildContext context, bool inverted) =>
    Theme.of(context)
        .textTheme
        .bodyLarge!
        .copyWith(
          color: inverted ? Colors.white : Colors.black,
        )
        .merge(isLargeScreen(context)
            ? VariableFontWeight.normal
            : VariableFontWeight.medium);

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
      Theme.of(context)
          .textTheme
          .labelLarge!
          .copyWith(fontSize: isSmallScreen(context) ? 16 : 14)
          .merge(VariableFontWeight.semiBold),
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
      Theme.of(context)
          .textTheme
          .labelLarge!
          .copyWith(fontSize: isSmallScreen(context) ? 16 : 14)
          .merge(
            isMain ? VariableFontWeight.semiBold : VariableFontWeight.medium,
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
      Theme.of(context)
          .textTheme
          .labelLarge!
          .copyWith(fontSize: isSmallScreen(context) ? 16 : 14)
          .merge(VariableFontWeight.semiBold),
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
