// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/main.dart';
import 'package:prototype/theme/theme.dart';
import 'package:provider/provider.dart';

import 'conversation_list_cubit.dart';
import 'create_conversation_view.dart';

final _log = Logger("ConversationListFooter");

class ConversationListFooter extends StatelessWidget {
  const ConversationListFooter({super.key});

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return SafeArea(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.end,
        children: [
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(Icons.person, size: 20),
            onPressed: () => _addConnection(context),
            label: Text(loc.conversationList_newConnection),
          ),
          TextButton.icon(
            style: textButtonStyle(context),
            icon: const Icon(Icons.notes, size: 20),
            onPressed: () => _addConversation(context),
            label: Text(loc.conversationList_newConversation),
          ),
        ],
      ),
    );
  }

  void _addConnection(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final loc = AppLocalizations.of(context);

    String? plaintextRes = await showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConnectionDialog_newConnectionTitle,
            loc.newConnectionDialog_newConnectionDescription,
            loc.newConnectionDialog_usernamePlaceholder,
            loc.newConnectionDialog_actionButton,
          ),
    );
    String plaintext = plaintextRes?.trim().toLowerCase() ?? "";

    if (plaintext.isNotEmpty) {
      try {
        final conversationId = await conversationListCubit.createConnection(
          handle: UiUserHandle(plaintext: plaintext),
        );
        _log.info(
          "A new 1:1 connection with user '$plaintext' was created: "
          "conversationId = $conversationId",
        );
      } catch (e) {
        if (context.mounted) {
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConnectionDialog_error(plaintext, e),
          );
        }
      }
    }
  }

  void _addConversation(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final loc = AppLocalizations.of(context);
    String? groupNameRes = await showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConversationDialog_newConversationTitle,
            loc.newConversationDialog_newConversationDescription,
            loc.newConversationDialog_conversationNamePlaceholder,
            loc.newConversationDialog_actionButton,
          ),
    );
    String groupName = groupNameRes?.trim() ?? "";
    if (groupName.isNotEmpty) {
      try {
        final conversationId = await conversationListCubit.createConversation(
          groupName: groupName,
        );
        _log.info(
          "A new group '$groupName' was created: "
          "conversationId = $conversationId",
        );
      } catch (e) {
        if (context.mounted) {
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConversationDialog_error(groupName, e),
          );
        }
      }
    }
  }
}
