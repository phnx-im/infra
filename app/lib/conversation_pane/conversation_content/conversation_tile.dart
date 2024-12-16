// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/conversation_content/display_message_tile.dart';
import 'package:prototype/conversation_pane/conversation_content/text_message_tile.dart';
import 'package:prototype/core/api/types.dart';

class ConversationTile extends StatelessWidget {
  const ConversationTile({
    required Key key,
    required this.message,
  }) : super(key: key);

  final UiConversationMessage message;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      title: Container(
        alignment: AlignmentDirectional.centerStart,
        child: switch (message.message) {
          UiMessage_ContentFlight(field0: final contentFlight) =>
            TextMessageTile(contentFlight, message.timestamp),
          UiMessage_Display(field0: final display) =>
            DisplayMessageTile(display, message.timestamp),
          UiMessage_Unsent(field0: final unsent) => Text(
              "⚠️ UNSENT MESSAGE ⚠️ $unsent",
              style: const TextStyle(color: Colors.red)),
        },
      ),
      selected: false,
    );
  }
}
