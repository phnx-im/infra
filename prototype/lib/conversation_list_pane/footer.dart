// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/create_view.dart';
import 'package:prototype/core_client.dart';

import '../main.dart';
import '../styles.dart';

class ConversationListFooter extends StatelessWidget {
  const ConversationListFooter({
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      alignment: AlignmentDirectional.topStart,
      padding: const EdgeInsets.fromLTRB(15, 15, 15, 30),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: [
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(
              Icons.person,
              size: 20,
            ),
            onPressed: () async {
              String connectionUsername = await showDialog(
                context: context,
                builder: (BuildContext context) => CreateView(
                    context,
                    "New connection",
                    "Enter the username to which you want to connect",
                    "USERNAME",
                    "Connect"),
              );
              if (connectionUsername.isNotEmpty) {
                try {
                  await coreClient.createConnection(connectionUsername);
                } catch (e) {
                  showErrorBanner(context,
                      'The user $connectionUsername could not be found');
                }
              }
            },
            label: const Text('New connection'),
          ),
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(
              Icons.notes,
              size: 20,
            ),
            onPressed: () async {
              String newGroup = await showDialog(
                  context: context,
                  builder: (BuildContext context) => CreateView(
                      context,
                      "New conversation",
                      "Choose a name for the new conversation",
                      "CONVERSATION NAME",
                      "Create conversation"));
              if (newGroup.isNotEmpty) {
                await coreClient.createConversation(newGroup);
                print('A new group was created: $newGroup');
              }
            },
            label: const Text('New conversation'),
          ),
        ],
      ),
    );
  }
}
