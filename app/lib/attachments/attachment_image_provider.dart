// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:prototype/core/core.dart';
import 'dart:ui' as ui;

/// Loads an attachment image from the database via an [AttachmentsRepository].
class AttachmentImageProvider extends ImageProvider<UiAttachment> {
  const AttachmentImageProvider({
    required this.attachment,
    required this.attachmentsRepository,
  });

  final UiAttachment attachment;
  final AttachmentsRepository attachmentsRepository;

  @override
  Future<UiAttachment> obtainKey(ImageConfiguration configuration) {
    return SynchronousFuture<UiAttachment>(attachment);
  }

  @override
  ImageStreamCompleter loadImage(
    UiAttachment key,
    ImageDecoderCallback decode,
  ) {
    debugPrint("Loading attachment image '$key'...");
    final chunkEvents = StreamController<ImageChunkEvent>();
    return MultiFrameImageStreamCompleter(
      codec: attachmentsRepository
          .loadAttachment(attachmentId: key.attachmentId)
          .catchError((Object e, StackTrace stack) {
            scheduleMicrotask(() {
              PaintingBinding.instance.imageCache.evict(key);
            });
            return Future<Uint8List>.error(e, stack);
          })
          .whenComplete(chunkEvents.close)
          .then<ui.ImmutableBuffer>(ui.ImmutableBuffer.fromUint8List)
          .then<ui.Codec>(decode),
      chunkEvents: chunkEvents.stream,
      scale: 1.0,
      debugLabel: '"key"',
      informationCollector:
          () => <DiagnosticsNode>[
            DiagnosticsProperty<ImageProvider>('Image provider', this),
            DiagnosticsProperty<UiAttachment>('Image key', key),
          ],
    );
  }

  @override
  bool operator ==(Object other) {
    if (other.runtimeType != runtimeType) {
      return false;
    }
    return other is AttachmentImageProvider && other.attachment == attachment;
  }

  @override
  int get hashCode => attachment.hashCode;

  @override
  String toString() =>
      '${objectRuntimeType(this, "AttachmentImageProvider")}("{$attachment.attachmentId}")';
}
