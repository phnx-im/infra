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

import 'chat_details_cubit.dart';
import 'report_spam_button.dart';

final _log = Logger('ConnectionDetails');

/// Details of a 1:1 connection
class ConnectionDetails extends StatelessWidget {
  const ConnectionDetails({super.key});

  @override
  Widget build(BuildContext context) {
    final chat = context.select((ChatDetailsCubit cubit) => cubit.state.chat);

    if (chat == null) {
      return const SizedBox.shrink();
    }

    final userId = switch (chat.chatType) {
      UiChatType_Connection(field0: final profile) => profile.userId,
      _ => null,
    };
    if (userId == null) {
      _log.warning("memberId is null in 1:1 connection details");
      return const SizedBox.shrink();
    }

    final isBlocked = chat.status == const UiChatStatus.blocked();

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

          isBlocked
              ? _UnblockConnectionButton(userId: userId)
              : _BlockConnectionButton(userId: userId),
          const SizedBox(height: Spacings.s),

          _DeleteConnectionButton(chatId: chat.id, contactName: chat.title),
          const SizedBox(height: Spacings.s),

          ReportSpamButton(userId: userId),
          const SizedBox(height: Spacings.s),
        ],
      ),
    );
  }
}

class _BlockConnectionButton extends StatelessWidget {
  const _BlockConnectionButton({required this.userId});

  final UiUserId userId;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return OutlinedButton(
      onPressed: () => _block(context, userId),
      child: Text(
        loc.blockConnectionButton_text,
        style: TextStyle(color: CustomColorScheme.of(context).function.danger),
      ),
    );
  }

  void _block(BuildContext context, UiUserId userId) async {
    final userCubit = context.read<UserCubit>();
    final loc = AppLocalizations.of(context);
    final confirmed = await showConfirmationDialog(
      context,
      title: loc.blockConnectionDialog_title,
      message: loc.blockConnectionDialog_content,
      positiveButtonText: loc.blockConnectionDialog_block,
      negativeButtonText: loc.blockConnectionDialog_cancel,
    );
    if (confirmed) {
      userCubit.blockContact(userId);
    }
  }
}

class _UnblockConnectionButton extends StatelessWidget {
  const _UnblockConnectionButton({required this.userId});

  final UiUserId userId;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return OutlinedButton(
      onPressed: () => unblockContactWithConfirmation(context, userId),
      child: Text(loc.unblockConnectionButton_text),
    );
  }
}

void unblockContactWithConfirmation(
  BuildContext context,
  UiUserId userId,
) async {
  final userCubit = context.read<UserCubit>();
  final loc = AppLocalizations.of(context);
  final confirmed = await showConfirmationDialog(
    context,
    title: loc.unblockConnectionDialog_title,
    message: loc.unblockConnectionDialog_content,
    positiveButtonText: loc.unblockConnectionDialog_unblock,
    negativeButtonText: loc.unblockConnectionDialog_cancel,
  );
  if (confirmed) {
    userCubit.unblockContact(userId);
  }
}

class _DeleteConnectionButton extends StatelessWidget {
  const _DeleteConnectionButton({
    required this.chatId,
    required this.contactName,
  });

  final ChatId chatId;
  final String contactName;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return OutlinedButton(
      onPressed: () => deleteChatWithConfirmation(context, chatId, contactName),
      child: Text(
        loc.deleteConnectionButton_text,
        style: TextStyle(color: CustomColorScheme.of(context).function.danger),
      ),
    );
  }
}

void deleteChatWithConfirmation(
  BuildContext context,
  ChatId chatId,
  String contactName,
) async {
  final userCubit = context.read<UserCubit>();
  final navigationCubit = context.read<NavigationCubit>();
  final loc = AppLocalizations.of(context);
  final confirmed = await showConfirmationDialog(
    context,
    title: loc.deleteConnectionDialog_title,
    message: loc.deleteConnectionDialog_content(contactName),
    positiveButtonText: loc.deleteConnectionDialog_delete,
    negativeButtonText: loc.deleteConnectionDialog_cancel,
  );
  if (confirmed) {
    userCubit.deleteChat(chatId);
    navigationCubit.closeChat();
  }
}
