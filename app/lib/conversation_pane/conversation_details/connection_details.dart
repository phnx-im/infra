// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/theme/theme.dart';
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

    return Center(
      child: Column(
        spacing: Spacings.l,
        children: [
          const SizedBox(height: Spacings.l),
          FutureUserAvatar(
            size: 64,
            profile: () => context
                .read<ConversationDetailsCubit>()
                .loadConversationUserProfile(),
          ),
          Text(
            conversation.title,
            style: labelStyle,
          ),
          Text(
            conversation.conversationType.description,
            style: labelStyle,
          ),
        ],
      ),
    );
  }
}
