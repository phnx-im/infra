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
    final chat = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.chat,
    );

    if (chat == null) {
      return const SizedBox.shrink();
    }

    final memberId = switch (chat.chatType) {
      UiChatType_Connection(field0: final profile) => profile.userId,
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
          UserAvatar(size: 128, displayName: chat.title, image: chat.picture),
          const SizedBox(height: Spacings.l),
          Text(style: Theme.of(context).textTheme.bodyLarge, chat.title),
          const SizedBox(height: Spacings.l),
          Text(
            chat.chatType.description,
            style: Theme.of(context).textTheme.bodyMedium,
          ),

          const Spacer(),

          _DeleteConnectionButton(conversationId: conversation.id),
          const SizedBox(height: Spacings.s),

          ReportSpamButton(userId: memberId),
          const SizedBox(height: Spacings.s),
        ],
      ),
    );
  }
}

class _DeleteConnectionButton extends StatelessWidget {
  const _DeleteConnectionButton({required this.conversationId});

  final ConversationId conversationId;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return OutlinedButton(
      onPressed: () => _delete(context, conversationId),
      child: Text(
        loc.deleteConnectionButton_text,
        style: TextStyle(color: CustomColorScheme.of(context).function.danger),
      ),
    );
  }

  void _delete(BuildContext context, ConversationId conversationId) async {
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    final loc = AppLocalizations.of(context);
    final confirmed = await showConfirmationDialog(
      context,
      title: loc.deleteConnectionDialog_title,
      message: loc.deleteConnectionDialog_content,
      positiveButtonText: loc.deleteConnectionDialog_delete,
      negativeButtonText: loc.deleteConnectionDialog_cancel,
    );
    if (confirmed) {
      userCubit.deleteConversation(conversationId);
      navigationCubit.closeConversation();
    }
  }
}
