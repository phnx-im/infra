// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/api/markdown.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'message_renderer.dart';

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
    final userName = context.select((UserCubit cubit) => cubit.state.userName);

    final isSender = contentMessage.sender == userName;

    return Column(
      children: [
        if (!isSender && flightPosition.isFirst) _sender(context, false),
        _messageSpace(
          context,
          isSender: isSender,
          flightPosition: flightPosition,
        ),
      ],
    );
  }

  Widget _messageSpace(
    BuildContext context, {
    required bool isSender,
    required UiFlightPosition flightPosition,
  }) {
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
                bottom: flightPosition.isLast ? 5 : 0),
            child: Column(
              crossAxisAlignment:
                  isSender ? CrossAxisAlignment.end : CrossAxisAlignment.start,
              children: [
                _textMessage(context, contentMessage.content.content, isSender),
                if (flightPosition.isLast) ...[
                  const SizedBox(height: 3),
                  _timestamp(context),
                ],
              ],
            ),
          ),
        ),
        if (!isSender) flex,
      ],
    );
  }

  Widget _sender(BuildContext context, bool isSender) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          FutureUserAvatar(
            profile: () =>
                context.read<UserCubit>().userProfile(contentMessage.sender),
          ),
          const SizedBox(width: 10),
          _username(isSender),
        ],
      ),
    );
  }

  Widget _timestamp(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 7.0),
      child: SelectionContainer.disabled(
        child: Text(
          timeString(timestamp),
          style: TextStyle(
            color: colorGreyDark,
            fontSize: isLargeScreen(context) ? 10 : 11,
            letterSpacing: -0.1,
          ).merge(VariableFontWeight.medium),
        ),
      ),
    );
  }

  String timeString(String time) {
    final t = DateTime.parse(time);
    // If the elapsed time is less than 60 seconds, show "now"
    if (DateTime.now().difference(t).inSeconds < 60) {
      return 'Now';
    }
    // If the elapsed time is less than 60 minutes, show the elapsed minutes
    if (DateTime.now().difference(t).inMinutes < 60) {
      return '${DateTime.now().difference(t).inMinutes}m ago';
    }
    // Otherwise show the time
    return '${t.hour}:${t.minute.toString().padLeft(2, '0')}';
  }

  Widget _textMessage(
      BuildContext context, MessageContent messageContent, bool isSender) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 1.0),
      child: Container(
        alignment: isSender
            ? AlignmentDirectional.topEnd
            : AlignmentDirectional.topStart,
        child: Container(
          padding: EdgeInsets.only(
            top: isLargeScreen(context) ? 1 : 4,
            right: isLargeScreen(context) ? 10 : 11,
            left: isLargeScreen(context) ? 10 : 11,
            bottom: isLargeScreen(context) ? 5 : 6,
          ),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(7),
            color: isSender ? colorDMB : colorDMBSuperLight,
          ),
          child: DefaultTextStyle.merge(
            style: messageTextStyle(context, isSender),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: messageContent.content
                  .map((inner) => buildBlockElement(inner.element, isSender))
                  .toList(),
            ),
          ),
        ),
      ),
    );
  }

  Widget _username(bool isSender) {
    return SelectionContainer.disabled(
      child: Text(
        isSender ? "You" : contentMessage.sender.split("@").firstOrNull ?? "",
        style: const TextStyle(
          color: colorDMB,
          fontSize: 12,
        ).merge(VariableFontWeight.semiBold),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
