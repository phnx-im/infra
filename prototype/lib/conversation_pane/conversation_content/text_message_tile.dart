// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/message_renderer.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';

class TextMessageTile extends StatefulWidget {
  final List<UiContentMessage> contentFlight;
  final String timestamp;
  const TextMessageTile(this.contentFlight, this.timestamp, {super.key});

  @override
  State<TextMessageTile> createState() => _TextMessageTileState();
}

class _TextMessageTileState extends State<TextMessageTile> {
  UiUserProfile? profile;

  @override
  void initState() {
    super.initState();
    context.coreClient.user
        .userProfile(userName: widget.contentFlight.last.sender)
        .then((p) {
      if (mounted) {
        setState(() {
          profile = p;
        });
      }
    });
  }

  bool isSender() {
    return widget.contentFlight.last.sender == context.coreClient.username;
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        if (!isSender()) _sender(),
        _messageSpace(),
      ],
    );
  }

  Widget _messageSpace() {
    // We use this to make an indent on the side of the receiver
    const flex = Flexible(child: SizedBox());
    return Row(
      mainAxisAlignment:
          isSender() ? MainAxisAlignment.end : MainAxisAlignment.start,
      children: [
        if (isSender()) flex,
        Flexible(
          flex: 5,
          child: Container(
            padding:
                const EdgeInsets.only(left: 0, right: 0, top: 5, bottom: 5),
            child: Column(
              crossAxisAlignment: isSender()
                  ? CrossAxisAlignment.end
                  : CrossAxisAlignment.start,
              children: [
                _textContent(context),
                const SizedBox(height: 3),
                _timestamp(),
              ],
            ),
          ),
        ),
        if (!isSender()) flex,
      ],
    );
  }

  Widget _sender() {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          _avatar(),
          const SizedBox(width: 10),
          _username(),
        ],
      ),
    );
  }

  Widget _avatar() {
    return FutureUserAvatar(
      profile: context.coreClient.user
          .userProfile(userName: widget.contentFlight.last.sender),
    );
  }

  Widget _timestamp() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 7.0),
      child: SelectionContainer.disabled(
        child: Text(
          timeString(widget.timestamp),
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

  Widget _textContent(BuildContext context) {
    final textMessages = widget.contentFlight
        .map((c) => _textMessage(context, c.content.body))
        .toList();
    return Column(
      children: textMessages,
    );
  }

  Widget _textMessage(BuildContext context, String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 1.0),
      child: Container(
        alignment: isSender()
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
            color: isSender() ? colorDMB : colorDMBSuperLight,
          ),
          child: RichText(
            text: buildTextSpanFromText(
                ["@Alice", "@Bob", "@Carol", "@Dave", "@Eve"],
                text,
                messageTextStyle(context, isSender()),
                HostWidget.richText),
            selectionRegistrar: SelectionContainer.maybeOf(context),
            selectionColor: Colors.blue.withOpacity(0.3),
            textWidthBasis: TextWidthBasis.longestLine,
          ),
        ),
      ),
    );
  }

  Widget _username() {
    return SelectionContainer.disabled(
      child: Text(
        isSender()
            ? "You"
            : widget.contentFlight.last.sender.split("@").firstOrNull ?? "",
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
