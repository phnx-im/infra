// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:prototype/core/core.dart';
import 'package:uuid/uuid.dart';

extension IntTestExtension on int {
  ConversationId conversationId() =>
      ConversationId(uuid: _intToUuidValue(this));

  ConversationMessageId conversationMessageId() =>
      ConversationMessageId(uuid: _intToUuidValue(this));

  /// Convert an int to a [ClientId].
  UiClientId clientId({String domain = "localhost"}) =>
      UiClientId(uuid: _intToUuidValue(this), domain: domain);
}

UuidValue _intToUuidValue(int value) {
  // Convert int to 16-byte array
  final bytes = Uint8List(16)
    ..buffer.asByteData().setInt64(0, value, Endian.little);
  return UuidValue.fromByteList(bytes);
}

class LocalFileComparatorWithThreshold extends LocalFileComparator {
  LocalFileComparatorWithThreshold(super.testFile, this.threshold);

  final double threshold;

  @override
  Future<bool> compare(Uint8List imageBytes, Uri golden) async {
    final result = await GoldenFileComparator.compareLists(
      imageBytes,
      await getGoldenBytes(golden),
    );
    if (!result.passed && result.diffPercent < threshold) {
      if ((result.diffPercent - threshold).abs() > 0.01) {
        final diff = (result.diffPercent * 10000.0).round() / 100.0;
        // ignore: avoid_print
        print(
          "Golden file comparison passed with $diff% difference, "
          "which is more than 1%pt under the configured threshold of ${threshold * 100}%. "
          "Consider making the threshold tighter.",
        );
      }
      return true;
    } else if (!result.passed) {
      final error = await generateFailureOutput(result, golden, basedir);
      throw FlutterError(error);
    } else {
      return result.passed;
    }
  }
}

/// Run `test` with a specific threshold for golden file comparisons.
Future<void> withThreshold(double threshold, AsyncCallback test) async {
  assert(goldenFileComparator is LocalFileComparator);
  final prevComparator = goldenFileComparator;
  final testUrl = (goldenFileComparator as LocalFileComparator).basedir;
  goldenFileComparator = LocalFileComparatorWithThreshold(
    // only the base dir is used from this URI, so pass a dummy file name
    Uri.parse('$testUrl/test.dart'),
    threshold,
  );
  try {
    await test();
  } catch (e) {
    rethrow;
  } finally {
    goldenFileComparator = prevComparator;
  }
}
