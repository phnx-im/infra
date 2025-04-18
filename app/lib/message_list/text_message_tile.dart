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
                ),
                if (flightPosition.isLast) ...[
                  const SizedBox(height: 3),
                  _Timestamp(timestamp),
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

class _Timestamp extends StatelessWidget {
  const _Timestamp(this.timestamp);

  final String timestamp;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 7.0),
      child: SelectionContainer.disabled(
        child: Text(
          _calcTimeString(timestamp),
          style: TextStyle(
            color: colorGreyDark,
            fontSize: isLargeScreen(context) ? 10 : 11,
            letterSpacing: -0.1,
          ).merge(VariableFontWeight.medium),
        ),
      ),
    );
  }
}

String _calcTimeString(String time) {
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

class _TextMessage extends StatelessWidget {
  const _TextMessage({required this.blockElements, required this.isSender});

  final List<RangedBlockElement> blockElements;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 1.0),
      child: Container(
        alignment:
            isSender
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

  final String sender;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          FutureUserAvatar(
            profile: () => context.read<UserCubit>().userProfile(sender),
          ),
          const SizedBox(width: 10),
          _Username(sender: sender, isSender: isSender),
        ],
      ),
    );
  }
}

class _Username extends StatelessWidget {
  const _Username({required this.sender, required this.isSender});

  final String sender;
  final bool isSender;

  @override
  Widget build(BuildContext context) {
    return SelectionContainer.disabled(
      child: Text(
        isSender ? "You" : sender.split("@").firstOrNull ?? "",
        style: const TextStyle(
          color: colorDMB,
          fontSize: 12,
        ).merge(VariableFontWeight.semiBold),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
