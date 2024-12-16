// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/main.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';

import 'conversation_list_cubit.dart';
import 'create_view.dart';

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
            onPressed: () => onNewConnection(context),
            label: const Text('New connection'),
          ),
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(
              Icons.notes,
              size: 20,
            ),
            onPressed: () => onNewConversation(context),
            label: const Text('New conversation'),
          ),
        ],
      ),
    );
  }

  void onNewConnection(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final navigationCubit = context.read<NavigationCubit>();

    String userName = await showDialog(
      context: context,
      builder: (BuildContext context) => CreateView(
          context,
          "New connection",
          "Enter the username to which you want to connect",
          "USERNAME",
          "Connect"),
    );
    if (userName.isEmpty) {
      return;
    }

    try {
      final conversationId = await conversationListCubit.createConnection(
        userName: userName,
      );
      navigationCubit.openConversation(conversationId);
    } catch (e) {
      if (context.mounted) {
        showErrorBanner(
          ScaffoldMessenger.of(context),
          'The user $userName could not be found',
        );
      }
    }
  }

  void onNewConversation(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final navigationCubit = context.read<NavigationCubit>();

    String groupName = await showDialog(
        context: context,
        builder: (BuildContext context) => CreateView(
            context,
            "New conversation",
            "Choose a name for the new conversation",
            "CONVERSATION NAME",
            "Create conversation"));
    if (groupName.isEmpty) {
      return;
    }

    final conversationId =
        await conversationListCubit.createConversation(groupName: groupName);
    navigationCubit.openConversation(conversationId);

    _log.info('A new group was created: $groupName');
  }
}
