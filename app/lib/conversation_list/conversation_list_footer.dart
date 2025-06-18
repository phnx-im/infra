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

    String? customError;

    validator(String? value) {
      final plaintext = value?.trim().toLowerCase();
      if (plaintext == null || plaintext.isEmpty) {
        return loc.newConnectionDialog_error_emptyHandle;
      }
      if (customError != null) {
        final error = customError;
        customError = null;
        return error;
      }
      UiUserHandle handle = UiUserHandle(plaintext: plaintext);
      return handle.validationError();
    }

    Future<String?> onAction(String input) async {
      final handle = UiUserHandle(plaintext: input.trim().toLowerCase());
      try {
        final conversationId = await conversationListCubit.createConnection(
          handle: handle,
        );
        if (context.mounted) {
          if (conversationId == null) {
            return loc.newConnectionDialog_error_handleNotFound(
              handle.plaintext,
            );
          }
          _log.info(
            "A new 1:1 connection with user '${handle.plaintext}' was created: "
            "conversationId = $conversationId",
          );
          Navigator.of(context).pop();
        }
      } catch (e) {
        _log.severe("Failed to create connection: $e");
        if (context.mounted) {
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConnectionDialog_error(handle.plaintext),
          );
        }
      }
      return null;
    }

    UiUserHandle? handle = await showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConnectionDialog_newConnectionTitle,
            loc.newConnectionDialog_newConnectionDescription,
            loc.newConnectionDialog_usernamePlaceholder,
            loc.newConnectionDialog_actionButton,
            validator: validator,
            onAction: onAction,
          ),
    );

    if (handle == null) {
      return;
    }

    try {
      final conversationId = await conversationListCubit.createConnection(
        handle: handle,
      );
      if (context.mounted) {
        if (conversationId == null) {
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConnectionDialog_error_handleNotFound(handle.plaintext),
          );
        } else {
          _log.info(
            "A new 1:1 connection with user '${handle.plaintext}' was created: "
            "conversationId = $conversationId",
          );
          Navigator.of(context).pop();
        }
      }
    } catch (e) {
      _log.severe("Failed to add user: $e");
      if (context.mounted) {
        showErrorBanner(
          ScaffoldMessenger.of(context),
          loc.newConnectionDialog_error(handle.plaintext),
        );
      }
    }
  }

  void _addConversation(BuildContext context) async {
    final conversationListCubit = context.read<ConversationListCubit>();
    final loc = AppLocalizations.of(context);

    validator(String? value) {
      final plaintext = value?.trim().toLowerCase();
      if (plaintext == null || plaintext.isEmpty) {
        return loc.newConversationDialog_error_emptyGroupName;
      }
      return null;
    }

    String? input = await showDialog(
      context: context,
      builder:
          (BuildContext context) => CreateConversationView(
            context,
            loc.newConversationDialog_newConversationTitle,
            loc.newConversationDialog_newConversationDescription,
            loc.newConversationDialog_conversationNamePlaceholder,
            loc.newConversationDialog_actionButton,
            validator: validator,
          ),
    );
    String name = input?.trim() ?? "";
    if (name.isNotEmpty) {
      try {
        final conversationId = await conversationListCubit.createConversation(
          groupName: name,
        );
        _log.info(
          "A new group '$name' was created: "
          "conversationId = $conversationId",
        );
      } catch (e) {
        if (context.mounted) {
          _log.severe("Failed to created conversation: $e");
          showErrorBanner(
            ScaffoldMessenger.of(context),
            loc.newConversationDialog_error(name),
          );
        }
      }
    }
  }
}
