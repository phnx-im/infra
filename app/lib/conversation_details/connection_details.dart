// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:air/l10n/app_localizations.dart';
import 'package:air/navigation/navigation_cubit.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user_cubit.dart';
import 'package:air/util/dialog.dart';
import 'package:flutter/material.dart';
import 'package:air/core/core.dart';
import 'package:air/theme/theme.dart';
import 'package:air/widgets/widgets.dart';
import 'package:logging/logging.dart';
import 'package:provider/provider.dart';

import 'conversation_details_cubit.dart';
import 'report_spam_button.dart';

final _log = Logger('ConnectionDetails');

/// Details of a 1:1 connection
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

    final loc = AppLocalizations.of(context);

    final memberId = switch (conversation.conversationType) {
      UiConversationType_Connection(field0: final profile) => profile.userId,
      _ => null,
    };
    if (memberId == null) {
      _log.warning("memberId is null in 1:1 connection details");
      return const SizedBox.shrink();
    }

    return Center(
      child: Column(
        children: [
          const SizedBox(height: Spacings.l),
          UserAvatar(
            size: 128,
            displayName: conversation.title,
            image: conversation.picture,
          ),
          const SizedBox(height: Spacings.l),
          Text(
            style: Theme.of(context).textTheme.bodyLarge,
            conversation.title,
          ),
          const SizedBox(height: Spacings.l),
          Text(
            conversation.conversationType.description,
            style: Theme.of(context).textTheme.bodyMedium,
          ),

          const Spacer(),

          ReportSpamButton(userId: memberId),
          const SizedBox(height: Spacings.s),

          const Spacer(),

          OutlinedButton(
            onPressed: () => _delete(context, conversation.id),
            child: Text(
              loc.deleteConnectionButton_text,
              style: TextStyle(
                color: CustomColorScheme.of(context).function.danger,
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _delete(BuildContext context, ConversationId conversationId) async {
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    if (await showConfirmationDialog(
      context,
      title: "Delete",
      message: // TODO: Localization
          "Are you sure you want to remove this connection? "
          "The message history will be also deleted.",
      positiveButtonText: "Delete",
      negativeButtonText: "Cancel",
    )) {
      userCubit.deleteConversation(conversationId);
      navigationCubit.closeConversation();
    }
  }
}
