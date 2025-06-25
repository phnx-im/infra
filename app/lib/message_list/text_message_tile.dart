// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_blurhash/flutter_blurhash.dart';
import 'package:logging/logging.dart';
import 'package:prototype/attachments/attachments.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/message_list/timestamp.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';

import 'message_renderer.dart';

const double largeCornerRadius = Spacings.s;
const double smallCornerRadius = Spacings.xxxs;
const double messageHorizontalPadding = Spacings.xs;
const double messageVerticalPadding = Spacings.xxs;

final _log = Logger('TextMessageTile');

class TextMessageTile extends StatelessWidget {
  const TextMessageTile({
    required this.contentMessage,
    required this.timestamp,
    required this.flightPosition,
    super.key,
  });

  final UiContentMessage contentMessage;
  final String timestamp;
  final UiFlightPosition flightPosition;

  @override
  Widget build(BuildContext context) {
    final userId = context.select((UserCubit cubit) => cubit.state.userId);
    final isSender = contentMessage.sender == userId;

    return Column(
      children: [
        if (!isSender && flightPosition.isFirst)
          _Sender(sender: contentMessage.sender, isSender: false),
        _MessageView(
          contentMessage: contentMessage,
          timestamp: timestamp,
          isSender: isSender,
          flightPosition: flightPosition,
        ),
      ],
    );
  }
}

class _MessageView extends StatelessWidget {
  const _MessageView({
    required this.contentMessage,
    required this.timestamp,
    required this.flightPosition,
    required this.isSender,
  });

  final UiContentMessage contentMessage;
  final String timestamp;
  final UiFlightPosition flightPosition;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    // We use this to make an indent on the side of the receiver
    const flex = Flexible(child: SizedBox.shrink());

    return Row(
      mainAxisAlignment:
          isSender ? MainAxisAlignment.end : MainAxisAlignment.start,
      children: [
        if (isSender) flex,
        Flexible(
          flex: 5,
          child: Container(
            padding: EdgeInsets.only(
              top: flightPosition.isFirst ? 5 : 0,
              bottom: flightPosition.isLast ? 5 : 0,
            ),
            child: Column(
              crossAxisAlignment:
                  isSender ? CrossAxisAlignment.end : CrossAxisAlignment.start,
              children: [
                _MessageContent(
                  content: contentMessage.content,
                  isSender: isSender,
                  flightPosition: flightPosition,
                ),
                if (flightPosition.isLast) ...[
                  const SizedBox(height: 2),
                  Timestamp(timestamp),
                ],
              ],
            ),
          ),
        ),
        if (!isSender) flex,
      ],
    );
  }
}

class _MessageContent extends StatelessWidget {
  const _MessageContent({
    required this.content,
    required this.isSender,
    required this.flightPosition,
  });

  final UiMimiContent content;
  final bool isSender;
  final UiFlightPosition flightPosition;

  // Calculate radii
  Radius _r(bool b) {
    return Radius.circular(b ? largeCornerRadius : smallCornerRadius);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 1.5),
      child: Container(
        alignment:
            isSender
                ? AlignmentDirectional.topEnd
                : AlignmentDirectional.topStart,
        child: Container(
          padding: const EdgeInsets.symmetric(
            horizontal: messageHorizontalPadding,
            vertical: messageVerticalPadding,
          ),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.only(
              topLeft: _r(isSender || flightPosition.isFirst),
              topRight: _r(!isSender || flightPosition.isFirst),
              bottomLeft: _r(isSender || flightPosition.isLast),
              bottomRight: _r(!isSender || flightPosition.isLast),
            ),
            color: isSender ? colorDMB : colorDMBSuperLight,
          ),
          child: DefaultTextStyle.merge(
            style: messageTextStyle(context, isSender),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (content.attachments.firstOrNull != null)
                  content.attachments.first.blurhash == null
                      ? _FileAttachmentContent(
                        attachment: content.attachments.first,
                        isSender: isSender,
                      )
                      : _ImageAttachmentContent(
                        attachment: content.attachments.first,
                        blurhash: content.attachments.first.blurhash!,
                        isSender: isSender,
                      ),
                ...(content.content?.elements ?? []).map(
                  (inner) => buildBlockElement(inner.element, isSender),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Sender extends StatelessWidget {
  const _Sender({required this.sender, required this.isSender});

  final UiUserId sender;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    final profile = context.select(
      (UsersCubit cubit) => cubit.state.profile(userId: sender),
    );

    return Padding(
      padding: const EdgeInsets.only(bottom: 4.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          UserAvatar(
            displayName: profile.displayName,
            image: profile.profilePicture,
          ),
          const SizedBox(width: 10),
          _DisplayName(displayName: profile.displayName, isSender: isSender),
        ],
      ),
    );
  }
}

class _DisplayName extends StatelessWidget {
  const _DisplayName({required this.displayName, required this.isSender});

  final String displayName;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    return SelectionContainer.disabled(
      child: Text(
        isSender ? "You" : displayName,
        style: const TextStyle(
          color: colorDMB,
          fontSize: 12,
        ).merge(VariableFontWeight.semiBold),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}

class _FileAttachmentContent extends StatelessWidget {
  const _FileAttachmentContent({
    required this.attachment,
    required this.isSender,
  });

  final UiAttachment attachment;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.file_present_sharp,
          size: 46,
          color: isSender ? Colors.white : Colors.black,
        ),
        const SizedBox(width: Spacings.xxs),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(attachment.filename),
            Text(loc.bytesToHumanReadable(attachment.size)),
          ],
        ),
      ],
    );
  }
}

class _ImageAttachmentContent extends StatelessWidget {
  const _ImageAttachmentContent({
    required this.attachment,
    required this.blurhash,
    required this.isSender,
  });

  final UiAttachment attachment;
  final String blurhash;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 300,
      child: AspectRatio(
        aspectRatio: 1.6,
        child: Stack(
          fit: StackFit.expand,
          children: [
            BlurHash(hash: blurhash),
            Image(
              image: AttachmentImageProvider(
                attachment: attachment,
                attachmentsRepository: RepositoryProvider.of(context),
              ),
              fit: BoxFit.cover,
              alignment: Alignment.center,
              errorBuilder: (context, error, stackTrace) {
                _log.severe('Failed to load attachment', error, stackTrace);
                return const Icon(Icons.error);
              },
            ),
          ],
        ),
      ),
    );
  }
}
