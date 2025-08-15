// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/ui/colors/palette.dart';
import 'package:prototype/ui/colors/semantic.dart';

class CustomColorScheme {
  final BackGroundBaseColors backgroundBase;
  final BackGroundElevatedColors backgroundElevated;
  final TextColors text;
  final SeparatorColors separator;
  final FillColors fill;
  final FunctionColors function;
  final MessageColors message;

  CustomColorScheme({
    required this.backgroundBase,
    required this.backgroundElevated,
    required this.text,
    required this.separator,
    required this.fill,
    required this.function,
    required this.message,
  });

  static CustomColorScheme of(BuildContext context) {
    return MediaQuery.platformBrightnessOf(context) == Brightness.dark
        ? darkTheme
        : lightTheme;
  }
}

final CustomColorScheme lightTheme = CustomColorScheme(
  backgroundBase: BackGroundBaseColors(
    primary: AppColors.neutral[0]!,
    secondary: AppColors.neutral[25]!,
    tertiary: AppColors.neutral[0]!,
    quaternary: AppColors.neutral[50]!,
  ),
  backgroundElevated: BackGroundElevatedColors(
    primary: AppColors.neutral[0]!,
    secondary: AppColors.neutral[25]!,
    tertiary: AppColors.neutral[0]!,
    quaternary: AppColors.neutral[50]!,
  ),
  text: TextColors(
    primary: AppColors.neutral[950]!.withValues(alpha: 0.95),
    secondary: AppColors.neutral[950]!.withValues(alpha: 0.85),
    tertiary: AppColors.neutral[950]!.withValues(alpha: 0.60),
    quaternary: AppColors.neutral[950]!.withValues(alpha: 0.40),
  ),
  separator: SeparatorColors(
    primary: AppColors.neutral[950]!.withValues(alpha: 0.20),
    secondary: AppColors.neutral[950]!.withValues(alpha: 0.10),
  ),
  fill: FillColors(
    primary: AppColors.neutral[950]!.withValues(alpha: 0.15),
    secondary: AppColors.neutral[950]!.withValues(alpha: 0.10),
    tertiary: AppColors.neutral[950]!.withValues(alpha: 0.05),
  ),
  function: FunctionColors(
    white: AppColors.neutral[0]!,
    black: AppColors.neutral[1000]!,
    toggleWhite: AppColors.neutral[0]!,
    toggleBlack: AppColors.neutral[100]!,
    success: AppColors.green[400]!,
    warning: AppColors.yellow[400]!,
    danger: AppColors.red[400]!,
    link: AppColors.blue[400]!,
  ),
  message: MessageColors(
    selfBackground: AppColors.neutral[600]!,
    otherBackground: AppColors.neutral[50]!,
    selfText: AppColors.neutral[0]!,
    otherText: AppColors.neutral[1000]!,
  ),
);

final CustomColorScheme darkTheme = CustomColorScheme(
  backgroundBase: BackGroundBaseColors(
    primary: AppColors.neutral[1000]!,
    secondary: AppColors.neutral[975]!,
    tertiary: AppColors.neutral[1000]!,
    quaternary: AppColors.neutral[950]!,
  ),
  backgroundElevated: BackGroundElevatedColors(
    primary: AppColors.neutral[950]!,
    secondary: AppColors.neutral[800]!,
    tertiary: AppColors.neutral[700]!,
    quaternary: AppColors.neutral[600]!,
  ),
  text: TextColors(
    primary: AppColors.neutral[50]!.withValues(alpha: 0.95),
    secondary: AppColors.neutral[50]!.withValues(alpha: 0.85),
    tertiary: AppColors.neutral[50]!.withValues(alpha: 0.60),
    quaternary: AppColors.neutral[50]!.withValues(alpha: 0.40),
  ),
  separator: SeparatorColors(
    primary: AppColors.neutral[50]!.withValues(alpha: 0.20),
    secondary: AppColors.neutral[50]!.withValues(alpha: 0.10),
  ),
  fill: FillColors(
    primary: AppColors.neutral[50]!.withValues(alpha: 0.20),
    secondary: AppColors.neutral[50]!.withValues(alpha: 0.15),
    tertiary: AppColors.neutral[50]!.withValues(alpha: 0.10),
  ),
  function: FunctionColors(
    white: AppColors.neutral[0]!,
    black: AppColors.neutral[1000]!,
    toggleWhite: AppColors.neutral[0]!,
    toggleBlack: AppColors.neutral[1000]!,
    success: AppColors.green[500]!,
    warning: AppColors.yellow[500]!,
    danger: AppColors.red[500]!,
    link: AppColors.blue[500]!,
  ),
  message: MessageColors(
    selfBackground: AppColors.neutral[300]!,
    otherBackground: AppColors.neutral[900]!,
    selfText: AppColors.neutral[900]!,
    otherText: AppColors.neutral[100]!,
  ),
);

final ColorScheme lightColorScheme = ColorScheme(
  brightness: Brightness.light,
  primary: lightTheme.text.primary,
  onPrimary: lightTheme.backgroundBase.primary,
  secondary: lightTheme.text.secondary,
  onSecondary: lightTheme.backgroundBase.primary,
  surface: lightTheme.backgroundBase.primary,
  onSurface: lightTheme.text.primary,
  error: lightTheme.function.danger,
  onError: lightTheme.text.primary,
);

final ColorScheme darkColorScheme = ColorScheme(
  brightness: Brightness.dark,
  primary: darkTheme.text.primary,
  onPrimary: darkTheme.backgroundBase.primary,
  secondary: darkTheme.text.secondary,
  onSecondary: darkTheme.backgroundBase.primary,
  surface: darkTheme.backgroundBase.primary,
  onSurface: darkTheme.text.primary,
  error: darkTheme.function.danger,
  onError: darkTheme.text.primary,
);
