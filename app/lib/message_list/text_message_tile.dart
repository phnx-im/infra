// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/attachments/attachments.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/message_list/timestamp.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/ui/typography/font_size.dart';
import 'package:prototype/ui/typography/monospace.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';

import 'message_renderer.dart';

const double largeCornerRadius = Spacings.sm;
const double smallCornerRadius = Spacings.xxs;
const double messageHorizontalPadding = Spacings.xs;
const double messageVerticalPadding = Spacings.xxs;

const _messagePadding = EdgeInsets.symmetric(
  horizontal: messageHorizontalPadding,
  vertical: messageVerticalPadding,
);

class TextMessageTile extends StatelessWidget {
  const TextMessageTile({
    required this.messageId,
    required this.contentMessage,
    required this.timestamp,
    required this.flightPosition,
    required this.status,
    super.key,
  });

  final ConversationMessageId messageId;
  final UiContentMessage contentMessage;
  final String timestamp;
  final UiFlightPosition flightPosition;
  final UiMessageStatus status;

  @override
  Widget build(BuildContext context) {
    final userId = context.select((UserCubit cubit) => cubit.state.userId);
    final isSender = contentMessage.sender == userId;

    return Column(
      children: [
        if (!isSender && flightPosition.isFirst)
          _Sender(sender: contentMessage.sender, isSender: false),
        _MessageView(
          messageId: messageId,
          contentMessage: contentMessage,
          timestamp: timestamp,
          isSender: isSender,
          flightPosition: flightPosition,
          status: status,
        ),
      ],
    );
  }
}

class _MessageView extends StatelessWidget {
  const _MessageView({
    required this.messageId,
    required this.contentMessage,
    required this.timestamp,
    required this.flightPosition,
    required this.isSender,
    required this.status,
  });

  final ConversationMessageId messageId;
  final UiContentMessage contentMessage;
  final String timestamp;
  final UiFlightPosition flightPosition;
  final bool isSender;
  final UiMessageStatus status;

  @override
  Widget build(BuildContext context) {
    // We use this to make an indent on the side of the receiver
    const flex = Flexible(child: SizedBox.shrink());

    final showMessageStatus =
        isSender && flightPosition.isLast && status != UiMessageStatus.sending;

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
                InkWell(
                  mouseCursor: SystemMouseCursors.basic,
                  onLongPress:
                      () => context
                          .read<ConversationDetailsCubit>()
                          .editMessage(messageId: messageId),
                  child: _MessageContent(
                    content: contentMessage.content,
                    isSender: isSender,
                    flightPosition: flightPosition,
                    isEdited: contentMessage.edited,
                  ),
                ),
                if (flightPosition.isLast) ...[
                  const SizedBox(height: 2),
                  Row(
                    mainAxisAlignment:
                        isSender
                            ? MainAxisAlignment.end
                            : MainAxisAlignment.start,
                    children: [
                      const SizedBox(width: Spacings.s),
                      Timestamp(timestamp),
                      if (showMessageStatus)
                        const SizedBox(width: Spacings.xxxs),
                      if (showMessageStatus)
                        DoubleCheckIcon(
                          size: LabelFontSize.small2.size,
                          singleCheckIcon: status == UiMessageStatus.sent,
                          inverted: status == UiMessageStatus.read,
                        ),
                      const SizedBox(width: Spacings.xs),
                    ],
                  ),
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
    required this.isEdited,
  });

  final UiMimiContent content;
  final bool isSender;
  final UiFlightPosition flightPosition;
  final bool isEdited;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: 1.5),
      child: Container(
        alignment:
            isSender
                ? AlignmentDirectional.topEnd
                : AlignmentDirectional.topStart,
        child: Container(
          decoration: BoxDecoration(
            borderRadius: _messageBorderRadius(isSender, flightPosition),
            color:
                isSender
                    ? customColors(context).message.selfBackground
                    : customColors(context).message.otherBackground,
          ),
          child: DefaultTextStyle.merge(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                if (content.attachments.firstOrNull case final attachment?)
                  switch (attachment.imageMetadata) {
                    null => _FileAttachmentContent(
                      attachment: attachment,
                      isSender: isSender,
                    ),
                    final imageMetadata => _ImageAttachmentContent(
                      attachment: attachment,
                      imageMetadata: imageMetadata,
                      isSender: isSender,
                      flightPosition: flightPosition,
                      hasMessage: content.content?.elements.isNotEmpty ?? false,
                    ),
                  },
                ...(content.content?.elements ?? []).map(
                  (inner) => Padding(
                    padding: _messagePadding.copyWith(
                      bottom: isEdited ? 0 : null,
                    ),
                    child: buildBlockElement(context, inner.element, isSender),
                  ),
                ),
                if (isEdited)
                  Padding(
                    padding: _messagePadding.copyWith(top: 0),
                    child: Text(
                      loc.textMessage_edited,
                      style: Theme.of(context).textTheme.bodySmall!.copyWith(
                        color:
                            isSender
                                ? customColors(context).text.quaternary
                                : customColors(context).text.tertiary,
                      ),
                    ),
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
      padding: const EdgeInsets.only(top: Spacings.xs, bottom: Spacings.xxs),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          UserAvatar(
            displayName: profile.displayName,
            image: profile.profilePicture,
            size: Spacings.m,
          ),
          const SizedBox(width: Spacings.xs),
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
    final text = isSender ? "You" : displayName;
    final textUpper = text.toUpperCase();
    return SelectionContainer.disabled(
      child: Text(
        textUpper,
        style: TextStyle(
          color: customColors(context).text.tertiary,
          fontSize: LabelFontSize.small2.size,
          fontWeight: FontWeight.w100,
          fontFamily: getSystemMonospaceFontFamily(),
          letterSpacing: 1.2,
        ),
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

    return Padding(
      padding: _messagePadding,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.file_present_sharp,
            size: 46,
            color:
                isSender
                    ? customColors(context).message.selfText
                    : customColors(context).message.otherText,
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
      ),
    );
  }
}

class _ImageAttachmentContent extends StatelessWidget {
  _ImageAttachmentContent({
    required this.attachment,
    required this.imageMetadata,
    required this.isSender,
    required this.flightPosition,
    required this.hasMessage,
  });

  final UiAttachment attachment;
  final UiImageMetadata imageMetadata;
  final bool isSender;
  final UiFlightPosition flightPosition;
  final bool hasMessage;

  final overlayController = OverlayPortalController();

  @override
  Widget build(BuildContext context) {
    return OverlayPortal(
      controller: overlayController,
      overlayChildBuilder:
          (BuildContext context) => _ImagePreview(
            attachment: attachment,
            imageMetadata: imageMetadata,
            isSender: isSender,
            overlayController: overlayController,
          ),
      child: GestureDetector(
        onTap: () {
          overlayController.show();
        },
        child: ClipRRect(
          borderRadius: _messageBorderRadius(
            isSender,
            flightPosition,
            stackedOnTop: hasMessage,
          ),
          child: Container(
            constraints: const BoxConstraints(maxHeight: 300),
            child: AttachmentImage(
              attachment: attachment,
              imageMetadata: imageMetadata,
              isSender: isSender,
              fit: BoxFit.cover,
            ),
          ),
        ),
      ),
    );
  }
}

class _ImagePreview extends StatelessWidget {
  const _ImagePreview({
    required this.attachment,
    required this.imageMetadata,
    required this.isSender,
    required this.overlayController,
  });

  final UiAttachment attachment;
  final UiImageMetadata imageMetadata;
  final bool isSender;
  final OverlayPortalController overlayController;

  @override
  Widget build(BuildContext context) {
    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event.logicalKey == LogicalKeyboardKey.escape &&
            event is KeyDownEvent) {
          overlayController.hide();
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        child: Container(
          height: MediaQuery.of(context).size.height,
          width: MediaQuery.of(context).size.width,
          color: customColors(context).backgroundBase.primary,
          child: Column(
            children: [
              AppBar(
                leading: const SizedBox.shrink(),
                actions: [
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () {
                      overlayController.hide();
                    },
                  ),
                  const SizedBox(width: Spacings.s),
                ],
                title: Text(attachment.filename),
                centerTitle: true,
              ),
              Expanded(
                child: Center(
                  child: Padding(
                    padding: const EdgeInsets.only(
                      bottom: Spacings.l,
                      left: Spacings.s,
                      right: Spacings.s,
                    ),
                    child: AttachmentImage(
                      attachment: attachment,
                      imageMetadata: imageMetadata,
                      isSender: isSender,
                      fit: BoxFit.contain,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

BorderRadius _messageBorderRadius(
  bool isSender,
  UiFlightPosition flightPosition, {
  bool stackedOnTop = false,
}) {
  // Calculate radii
  Radius r(bool b) =>
      Radius.circular(b ? largeCornerRadius : smallCornerRadius);

  return BorderRadius.only(
    topLeft: r(isSender || flightPosition.isFirst),
    topRight: r(!isSender || flightPosition.isFirst),
    bottomLeft:
        !stackedOnTop ? r(isSender || flightPosition.isLast) : Radius.zero,
    bottomRight:
        !stackedOnTop ? r(!isSender || flightPosition.isLast) : Radius.zero,
  );
}
