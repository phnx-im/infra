// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

/// The threshold for golden file comparisons to pass (between 0 and 1 as percent)
const goldenThreshold = 0.022;

/// The physical size of the screen in the test environment
const pixel8ScreenSize = Size(1080, 2400);

Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  setUpAll(() async {
    final binding = TestWidgetsFlutterBinding.ensureInitialized();
    await _loadFonts();
    await _setGoldenFileComparatorWithThreshold(goldenThreshold);
    await _setPhysicalScreenSize(binding, pixel8ScreenSize);
  });

  await testMain();
}

Future<void> _loadFonts() async {
  final fonts = {
    "InterEmbedded": "assets/fonts/inter.ttf",
    "MaterialIcons": "fonts/MaterialIcons-Regular.otf",
  };
  for (final entry in fonts.entries) {
    final font = rootBundle.load(entry.value);
    final FontLoader fontLoader = FontLoader(entry.key)..addFont(font);
    await fontLoader.load();
  }
}

Future<void> _setGoldenFileComparatorWithThreshold(double threshold) async {
  assert(goldenFileComparator is LocalFileComparator);
  final testUrl = (goldenFileComparator as LocalFileComparator).basedir;
  goldenFileComparator = _LocalFileComparatorWithThreshold(
    // only the base dir is used from this URI, so pass a dummy file name
    Uri.parse('$testUrl/test.dart'), threshold,
  );
}

class _LocalFileComparatorWithThreshold extends LocalFileComparator {
  _LocalFileComparatorWithThreshold(super.testFile, this.threshold);

  final double threshold;

  @override
  Future<bool> compare(Uint8List imageBytes, Uri golden) async {
    final result = await GoldenFileComparator.compareLists(
      imageBytes,
      await getGoldenBytes(golden),
    );
    if (!result.passed && result.diffPercent < threshold) {
      return true;
    } else if (!result.passed) {
      final error = await generateFailureOutput(result, golden, basedir);
      throw FlutterError(error);
    } else {
      return result.passed;
    }
  }
}

_setPhysicalScreenSize(
  TestWidgetsFlutterBinding binding,
  Size pixel8screenSize,
) {
  // set physical size of the screen
  binding.platformDispatcher.views.first.physicalSize = pixel8ScreenSize;
  addTearDown(() {
    binding.platformDispatcher.views.first.resetPhysicalSize();
  });
}
