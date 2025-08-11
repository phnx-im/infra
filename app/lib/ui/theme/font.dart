// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/ui/typography/font_size.dart';

const _letterSpacing = 0.0;

final customTextScheme = TextTheme(
  displayLarge: TextStyle(
    fontSize: HeaderFontSize.h1.size,
    letterSpacing: _letterSpacing,
  ),
  displayMedium: TextStyle(
    fontSize: HeaderFontSize.h2.size,
    letterSpacing: _letterSpacing,
  ),
  displaySmall: TextStyle(
    fontSize: HeaderFontSize.h3.size,
    letterSpacing: _letterSpacing,
  ),
  headlineLarge: TextStyle(
    fontSize: HeaderFontSize.h4.size,
    letterSpacing: _letterSpacing,
  ),
  headlineMedium: TextStyle(
    fontSize: HeaderFontSize.h5.size,
    letterSpacing: _letterSpacing,
  ),
  headlineSmall: TextStyle(
    fontSize: HeaderFontSize.h6.size,
    letterSpacing: _letterSpacing,
  ),
  titleLarge: TextStyle(
    fontSize: HeaderFontSize.h4.size,
    letterSpacing: _letterSpacing,
  ),
  titleMedium: TextStyle(
    fontSize: LabelFontSize.small1.size,
    letterSpacing: _letterSpacing,
  ),
  titleSmall: TextStyle(
    fontSize: LabelFontSize.small2.size,
    letterSpacing: _letterSpacing,
  ),
  bodyLarge: TextStyle(
    fontSize: BodyFontSize.base.size,
    letterSpacing: _letterSpacing,
  ),
  bodyMedium: TextStyle(
    fontSize: BodyFontSize.small1.size,
    letterSpacing: _letterSpacing,
  ),
  bodySmall: TextStyle(
    fontSize: BodyFontSize.small2.size,
    letterSpacing: _letterSpacing,
  ),
  labelLarge: TextStyle(
    fontSize: LabelFontSize.small1.size,
    letterSpacing: _letterSpacing,
  ),
  labelMedium: TextStyle(
    fontSize: LabelFontSize.small2.size,
    letterSpacing: _letterSpacing,
  ),
  labelSmall: TextStyle(
    fontSize: LabelFontSize.small2.size,
    letterSpacing: _letterSpacing,
  ),
);
