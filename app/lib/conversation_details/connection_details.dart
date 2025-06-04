// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'conversation_details_cubit.dart';

// Details of a 1:1 connection
class ConnectionDetails extends StatelessWidget {
  const ConnectionDetails({super.key});

  @override
  Widget build(BuildContext context) {
    final conversation = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.conversation,
    );

    if (conversation == null) {
      return const SizedBox.shrink();
    }

    return Align(
      alignment: Alignment.topCenter,
      child: Container(
        constraints: isPointer() ? const BoxConstraints(maxWidth: 800) : null,
        padding: const EdgeInsets.all(Spacings.s),
        child: Column(
          children: [
            UserAvatar(
              size: 100,
              displayName: conversation.title,
              image: conversation.picture,
            ),
            const SizedBox(height: Spacings.m),
            Text(
              conversation.title,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            const SizedBox(height: Spacings.s),
            Text(
              conversation.conversationType.description,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            const Spacer(),
            OutlinedButton(
              onPressed: () => _delete(context, conversation.id),
              style: dangerButtonStyle(context),
              child: const Text('Delete'),
            ),
          ],
        ),
      ),
    );
  }

  void _delete(BuildContext context, ConversationId conversationId) async {
    bool confirmed = await showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text("Delete"),
          content: const Text(
            "Are you sure you want to remove this connection? The message history will be deleted.",
          ),
          actions: [
            TextButton(
              onPressed: () {
                Navigator.of(context).pop(false);
              },
              style: textButtonStyle(context),
              child: const Text("Cancel"),
            ),
            TextButton(
              onPressed: () async {
                if (context.mounted) {
                  Navigator.of(context).pop(true);
                }
              },
              style: textButtonStyle(context),
              child: const Text("Delete connection"),
            ),
          ],
        );
      },
    );

    if (!confirmed) {
      return;
    }

    if (context.mounted) {
      await context.read<UserCubit>().deleteConversation(conversationId);
    }
    if (context.mounted) {
      Navigator.of(context).pop(true);
    }
  }
}
