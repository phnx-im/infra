// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

          ReportSpamButton(userId: memberId),
          const SizedBox(height: Spacings.s),
        ],
      ),
    );
  }
}
