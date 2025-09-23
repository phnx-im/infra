import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import '../test/product_shots/product_shots.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  goldenFileComparator = WrappingComparatorWithSuffix(goldenFileComparator);

  run(outputBase: "product_shots");
}

/// Adds platform suffix to golden file names
class WrappingComparatorWithSuffix extends GoldenFileComparator {
  WrappingComparatorWithSuffix(this.comparator);

  final GoldenFileComparator comparator;

  String _platformSuffix() {
    if (Platform.isMacOS) return '.macos';
    if (Platform.isWindows) return '.windows';
    if (Platform.isLinux) return '.linux';
    if (Platform.isAndroid) return '.android';
    if (Platform.isIOS) return '.ios';
    return '';
  }

  @override
  Uri getTestUri(Uri key, int? version) {
    final path = key.toFilePath();
    final newPath = path.replaceFirst(
      RegExp(r'\.png$'),
      '${_platformSuffix()}.png',
    );
    return Uri.file(newPath);
  }

  @override
  Future<bool> compare(Uint8List imageBytes, Uri golden) =>
      comparator.compare(imageBytes, golden);

  @override
  Future<void> update(Uri golden, Uint8List imageBytes) =>
      comparator.update(golden, imageBytes);
}
