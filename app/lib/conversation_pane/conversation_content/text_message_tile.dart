// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/message_renderer.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/user_cubit.dart';
import 'package:provider/provider.dart';

class TextMessageTile extends StatelessWidget {
  final List<UiContentMessage> contentFlight;
  final String timestamp;
  const TextMessageTile(this.contentFlight, this.timestamp, {super.key});

  @override
  Widget build(BuildContext context) {
    final userName = context.select((UserCubit cubit) => cubit.state.userName);
    final isSender = contentFlight.last.sender == userName;

    return Column(
      children: [
        if (!isSender) _sender(context, false),
        _messageSpace(context, isSender),
      ],
    );
  }

  Widget _messageSpace(BuildContext context, bool isSender) {
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
            padding:
                const EdgeInsets.only(left: 0, right: 0, top: 5, bottom: 5),
            child: Column(
              crossAxisAlignment:
                  isSender ? CrossAxisAlignment.end : CrossAxisAlignment.start,
              children: [
                _textContent(context, isSender),
                const SizedBox(height: 3),
                _timestamp(context),
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
          _avatar(context),
          const SizedBox(width: 10),
          _username(isSender),
        ],
      ),
    );
  }

  Widget _avatar(BuildContext context) {
    return FutureUserAvatar(
      profile: () => context.coreClient.user
          .userProfile(userName: contentFlight.last.sender),
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
            fontVariations: variationMedium,
            letterSpacing: -0.1,
          ),
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

  Widget _textContent(BuildContext context, bool isSender) {
    final textMessages = contentFlight
        .map((c) => _textMessage(context, c.content.body, isSender))
        .toList();
    return Column(
      children: textMessages,
    );
  }

  Widget _textMessage(BuildContext context, String text, bool isSender) {
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
          child: RichText(
            text: buildTextSpanFromText(
                ["@Alice", "@Bob", "@Carol", "@Dave", "@Eve"],
                text,
                messageTextStyle(context, isSender),
                HostWidget.richText),
            selectionRegistrar: SelectionContainer.maybeOf(context),
            selectionColor: Colors.blue.withValues(alpha: 0.3),
            textWidthBasis: TextWidthBasis.longestLine,
          ),
        ),
      ),
    );
  }

  Widget _username(bool isSender) {
    return SelectionContainer.disabled(
      child: Text(
        isSender
            ? "You"
            : contentFlight.last.sender.split("@").firstOrNull ?? "",
        style: const TextStyle(
          color: colorDMB,
          fontVariations: variationSemiBold,
          fontSize: 12,
          letterSpacing: -0.2,
        ),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
