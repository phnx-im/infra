// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:applogic/applogic.dart';
import 'package:prototype/conversation_pane/conversation_content/display_message_tile.dart';
import 'package:prototype/core_client.dart';
import 'text_message_tile.dart';
import 'package:flutter/scheduler.dart';
import 'package:collection/collection.dart';

class ConversationContent extends StatefulWidget {
  final UiConversation? conversation;

  const ConversationContent({super.key, required this.conversation});

  @override
  State<ConversationContent> createState() => _ConversationContentState();
}

class _ConversationContentState extends State<ConversationContent> {
  final ScrollController _scrollController = ScrollController();
  List<UiConversationMessage> messages = [];
  UiConversation? _currentConversation;
  StreamSubscription<UiConversation>? _conversationListener;
  StreamSubscription<UiConversationMessage>? _messageListener;

  @override
  void initState() {
    super.initState();
    _conversationListener =
        coreClient.onConversationSwitch.listen(conversationListener);
    _messageListener = coreClient.onMessageUpdate.listen(messageListener);

    _currentConversation = widget.conversation;

    if (_currentConversation != null) {
      // Updates the messages and scrolls to the end of the conversation.
      updateMessages().then((_) => scrollToEnd());
    }
  }

  @override
  void dispose() {
    _conversationListener?.cancel();
    _messageListener?.cancel();
    super.dispose();
  }

  Future<void> updateMessages() async {
    if (_currentConversation != null) {
      final messages = await coreClient.user
          .getMessages(conversationId: _currentConversation!.id, lastN: 100);
      setState(() {
        print("Number of messages: ${messages.length}");
        this.messages = messages;
      });
    }
  }

  void conversationListener(UiConversation conversation) async {
    _currentConversation = conversation;
    messages = [];
    await updateMessages();
  }

  void messageListener(UiConversationMessage cm) {
    if (cm.conversationId.bytes.equals(_currentConversation!.id.bytes)) {
      setState(() {
        messages.add(cm);
        scrollToEnd();
      });
    } else {
      print('A message for another group was received');
    }
  }

  // Smooth scrolling to the end of the conversation
  void scrollToEnd() {
    setState(() {
      SchedulerBinding.instance.addPostFrameCallback((_) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeInOut,
        );
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: SelectionArea(
        child: ListView.builder(
          controller: _scrollController,
          padding: EdgeInsets.only(
            top: kToolbarHeight +
                MediaQuery.of(context)
                    .padding
                    .top, // Use the AppBar's height as padding
            left: 10,
          ),
          itemCount: messages.length,
          physics: const BouncingScrollPhysics(),
          itemBuilder: (BuildContext context, int index) {
            final message = messages[index];
            return ListTile(
              title: Container(
                margin: const EdgeInsets.symmetric(vertical: 10),
                alignment: AlignmentDirectional.centerStart,
                child: (message.message.when(
                  content: (content) =>
                      TextMessageTile(content, message.timestamp),
                  display: (display) =>
                      DisplayMessageTile(display, message.timestamp),
                  unsent: (unsent) => const Text(
                      "⚠️ UNSENT MESSAGE ⚠️ {unsent}",
                      style: TextStyle(color: Colors.red)),
                )),
              ),
              selected: false,
              focusColor: Colors.transparent,
              hoverColor: Colors.transparent,
            );
          },
        ),
      ),
    );
  }
}
