// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/main.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

import 'conversation_list_cubit.dart';
import 'create_conversation_view.dart';

final _log = Logger("ConversationListFooter");

class ConversationListFooter extends StatelessWidget {
  const ConversationListFooter({super.key});

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.end,
        children: [
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(Icons.person, size: 20),
            onPressed: () async {
              // Currently, we only support connections to the same domain.
              final domain = context.read<UserCubit>().state.userId.domain;

              final conversationListCubit =
                  context.read<ConversationListCubit>();
              String connectionUuid =
                  (await showDialog(
                    context: context,
                    builder:
                        (BuildContext context) => CreateConversationView(
                          context,
                          "New connection",
                          "Enter the unique ID (UUID) of the user you want to connect to",
                          "UUID",
                          "Connect",
                        ),
                  )).trim();

              if (connectionUuid.isNotEmpty) {
                final clientUuid = UuidValue.withValidation(
                  connectionUuid,
                  ValidationMode.nonStrict,
                );
                final connectionId = UiUserId(uuid: clientUuid, domain: domain);
                try {
                  await conversationListCubit.createConnection(
                    userId: connectionId,
                  );
                } catch (e) {
                  if (context.mounted) {
                    showErrorBanner(
                      ScaffoldMessenger.of(context),
                      'Failed to add user with UUID $connectionUuid',
                    );
                  }
                }
              }
            },
            label: const Text('New connection'),
          ),
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(Icons.notes, size: 20),
            onPressed: () async {
              final conversationListCubit =
                  context.read<ConversationListCubit>();
              String newGroup = await showDialog(
                context: context,
                builder:
                    (BuildContext context) => CreateConversationView(
                      context,
                      "New conversation",
                      "Choose a name for the new conversation",
                      "CONVERSATION NAME",
                      "Create conversation",
                    ),
              );
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
