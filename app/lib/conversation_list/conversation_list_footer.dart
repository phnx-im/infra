// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/main.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';

import 'conversation_list_cubit.dart';
import 'create_conversation_view.dart';

final _log = Logger("ConversationListFooter");

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
              final conversationListCubit =
                  context.read<ConversationListCubit>();
              String connectionUsername = await showDialog(
                context: context,
                builder: (BuildContext context) => CreateConversationView(
                    context,
                    "New connection",
                    "Enter the username to which you want to connect",
                    "USERNAME",
                    "Connect"),
              );
              if (connectionUsername.isNotEmpty) {
                try {
                  await conversationListCubit.createConnection(
                    userName: connectionUsername,
                  );
                } catch (e) {
                  if (context.mounted) {
                    showErrorBanner(
                      ScaffoldMessenger.of(context),
                      'The user $connectionUsername could not be found',
                    );
                  }
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
              final conversationListCubit =
                  context.read<ConversationListCubit>();
              String newGroup = await showDialog(
                  context: context,
                  builder: (BuildContext context) => CreateConversationView(
                      context,
                      "New conversation",
                      "Choose a name for the new conversation",
                      "CONVERSATION NAME",
                      "Create conversation"));
              if (newGroup.isNotEmpty) {
                await conversationListCubit.createConversation(
                  groupName: newGroup,
                );
                _log.info('A new group was created: $newGroup');
              }
            },
            label: const Text('New conversation'),
          ),
        ],
      ),
    );
  }
}
