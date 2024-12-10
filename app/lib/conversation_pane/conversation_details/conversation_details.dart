// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/conversation_details/connection_details.dart';
import 'package:prototype/conversation_pane/conversation_details/group_details.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';

class ConversationDetails extends StatefulWidget {
  const ConversationDetails({super.key});

  @override
  State<ConversationDetails> createState() => _ConversationDetailsState();
}

class _ConversationDetailsState extends State<ConversationDetails> {
  UiConversationDetails? _currentConversation;
  late StreamSubscription<UiConversationDetails> _conversationListener;

  @override
  void initState() {
    super.initState();
    final coreClient = context.coreClient;
    _conversationListener =
        coreClient.onConversationSwitch.listen(conversationListener);

    _currentConversation = coreClient.currentConversation;
  }

  @override
  void dispose() {
    _conversationListener.cancel();
    super.dispose();
  }

  void conversationListener(UiConversationDetails conversation) async {
    Navigator.of(context).pop();
    return;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: appBarBackButton(context),
        title: const Text("Details"),
      ),
      body: _currentConversation?.conversationType.when(
              unconfirmedConnection: (e) =>
                  ConnectionDetails(conversation: _currentConversation!),
              connection: (e) => ConnectionDetails(
                    conversation: _currentConversation!,
                  ),
              group: () => GroupDetails(
                    conversation: _currentConversation!,
                  )) ??
          Container(),
    );
  }
}
