// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/widgets.dart';

/// Font weight for variable fonts.
///
/// Misnomer: Technically, this is a font style.
class VariableFontWeight {
  /// Thin, the least thick.
  static const TextStyle w100 = TextStyle(
    fontWeight: FontWeight.w100,
    fontVariations: [FontVariation('wght', 100.0)],
  );

  /// Extra-light.
  static const TextStyle w200 = TextStyle(
    fontWeight: FontWeight.w200,
    fontVariations: [FontVariation('wght', 200.0)],
  );

  /// Light.
  static const TextStyle w300 = TextStyle(
    fontWeight: FontWeight.w300,
    fontVariations: [FontVariation('wght', 300.0)],
  );

  /// Normal / regular / plain.
  static const TextStyle w400 = TextStyle(
    fontWeight: FontWeight.w400,
    fontVariations: [FontVariation('wght', 400.0)],
  );

  /// Medium.
  static const TextStyle w500 = TextStyle(
    fontWeight: FontWeight.w500,
    fontVariations: [FontVariation('wght', 500.0)],
  );

  /// Semi-bold.
  static const TextStyle w600 = TextStyle(
    fontWeight: FontWeight.w600,
    fontVariations: [FontVariation('wght', 600.0)],
  );

  /// Bold.
  static const TextStyle w700 = TextStyle(
    fontWeight: FontWeight.w700,
    fontVariations: [FontVariation('wght', 700.0)],
  );

  /// Extra-bold.
  static const TextStyle w800 = TextStyle(
    fontWeight: FontWeight.w800,
    fontVariations: [FontVariation('wght', 800.0)],
  );

  /// Black, the most thick.
  static const TextStyle w900 = TextStyle(
    fontWeight: FontWeight.w900,
    fontVariations: [FontVariation('wght', 900.0)],
  );

  /// The default font weight.
  static const TextStyle normal = w400;

  /// Slightly heaver than normal.
  static const TextStyle medium = w500;

  /// Heaver than normal.
  static const TextStyle semiBold = w600;

  /// A commonly used font weight that is heavier than normal.
  static const TextStyle bold = w700;

  /// A list of all the font weights.
  static const List<TextStyle> values = [
    w100,
    w200,
    w300,
    w400,
    w500,
    w600,
    w700,
    w800,
    w900
  ];
}
