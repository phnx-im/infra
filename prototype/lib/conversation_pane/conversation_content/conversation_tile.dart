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
        margin: const EdgeInsets.symmetric(vertical: 10),
        alignment: AlignmentDirectional.centerStart,
        child: (message.message.when(
          content: (content) => TextMessageTile(content, message.timestamp),
          display: (display) => DisplayMessageTile(display, message.timestamp),
          unsent: (unsent) => const Text("⚠️ UNSENT MESSAGE ⚠️ {unsent}",
              style: TextStyle(color: Colors.red)),
        )),
      ),
      selected: false,
      focusColor: Colors.transparent,
      hoverColor: Colors.transparent,
    );
  }
}
