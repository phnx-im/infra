// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_blurhash/flutter_blurhash.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';

import 'attachment_image_provider.dart';

final _log = Logger('AttachmentImage');

/// An image that is loaded from the database via an [AttachmentsRepository].
///
/// During loading, image's blurhash is shown instead of the image.
class AttachmentImage extends StatefulWidget {
  const AttachmentImage({
    super.key,
    required this.attachment,
    required this.imageMetadata,
    required this.isSender,
    required this.fit,
  });

  final UiAttachment attachment;
  final UiImageMetadata imageMetadata;
  final bool isSender;
  final BoxFit fit;

  @override
  State<AttachmentImage> createState() => _AttachmentImageState();
}

class _AttachmentImageState extends State<AttachmentImage> {
  @override
  Widget build(BuildContext context) {
    return AspectRatio(
      aspectRatio: widget.imageMetadata.width / widget.imageMetadata.height,
      child: Stack(
        fit: StackFit.expand,
        children: [
          BlurHash(hash: widget.imageMetadata.blurhash),
          Image(
            image: AttachmentImageProvider(
              attachment: widget.attachment,
              attachmentsRepository: RepositoryProvider.of(context),
            ),
            loadingBuilder: loadingBuilder,
            fit: widget.fit,
            alignment: Alignment.center,
            errorBuilder: (context, error, stackTrace) {
              _log.severe('Failed to load attachment', error, stackTrace);
              return const Icon(Icons.error);
            },
          ),
        ],
      ),
    );
  }

  Widget loadingBuilder(
    BuildContext context,
    Widget child,
    ImageChunkEvent? loadingProgress,
  ) {
    if (loadingProgress == null) {
      return child;
    }
    return Center(
      child: CircularProgressIndicator(
        valueColor: const AlwaysStoppedAnimation<Color>(colorDMB),
        backgroundColor: Colors.transparent,
        value:
            loadingProgress.expectedTotalBytes != null
                ? loadingProgress.cumulativeBytesLoaded /
                    loadingProgress.expectedTotalBytes!
                : null,
      ),
    );
  }
}
