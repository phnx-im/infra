// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Same as [MemoryImage] but caches the result in memory under the given [tag]
class CachedMemoryImage extends ImageProvider<CachedMemoryImage> {
  const CachedMemoryImage(
    this.tag,
    this.bytes,
  );

  final String tag;
  final Uint8List bytes;

  @override
  ImageStreamCompleter loadImage(
    CachedMemoryImage key,
    ImageDecoderCallback decode,
  ) {
    return MultiFrameImageStreamCompleter(
      codec: _loadAsync(key, decode: decode),
      scale: 1.0,
      debugLabel: 'CachedMemoryImage($tag)',
    );
  }

  Future<ui.Codec> _loadAsync(
    CachedMemoryImage key, {
    required ImageDecoderCallback decode,
  }) async {
    return decode(await ui.ImmutableBuffer.fromUint8List(bytes));
  }

  @override
  Future<CachedMemoryImage> obtainKey(ImageConfiguration configuration) {
    return SynchronousFuture<CachedMemoryImage>(this);
  }

  @override
  bool operator ==(Object other) =>
      other.runtimeType == runtimeType &&
      other is CachedMemoryImage &&
      other.tag == tag;

  @override
  int get hashCode => tag.hashCode;

  @override
  String toString() => '${objectRuntimeType(this, 'CachedMemoryImage')}($tag)';
}
