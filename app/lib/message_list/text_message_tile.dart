// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/api/markdown.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/message_list/timestamp.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'message_renderer.dart';

const double largeCornerRadius = Spacings.s;
const double smallCornerRadius = Spacings.xxxs;
const double messageHorizontalPadding = Spacings.xs;
const double messageVerticalPadding = Spacings.xxs;

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
                _TextMessage(
                  blockElements: contentMessage.content.content.content,
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

class _TextMessage extends StatelessWidget {
  const _TextMessage({
    required this.blockElements,
    required this.isSender,
    required this.flightPosition,
  });

  final List<RangedBlockElement> blockElements;
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
              children:
                  blockElements
                      .map(
                        (inner) => buildBlockElement(inner.element, isSender),
                      )
                      .toList(),
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
