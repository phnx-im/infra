// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/conversation_pane/conversation_details/conversation_details_cubit.dart';
import 'package:prototype/core/api/types.dart';

import 'conversation_tile.dart';
import 'message_cubit.dart';

final ScrollPhysics _scrollPhysics =
    (Platform.isAndroid || Platform.isWindows || Platform.isLinux)
        ? const ClampingScrollPhysics()
        : const BouncingScrollPhysics()
            .applyTo(const AlwaysScrollableScrollPhysics());

class MessageListView extends StatelessWidget {
  const MessageListView({super.key});

  @override
  Widget build(BuildContext context) {
    final messagesCount = context.select(
      (ConversationDetailsCubit cubit) =>
          cubit.state.conversation?.messagesCount,
    );

    if (messagesCount == null) {
      return const SizedBox.shrink();
    }

    return Expanded(
      child: SelectionArea(
        child: ListView.custom(
          physics: _scrollPhysics,
          reverse: true,
          childrenDelegate: SliverChildBuilderDelegate(
            (context, index) {
              final messageId = context
                  .read<ConversationDetailsCubit>()
                  .messageIdFromRevOffset(index);
              return messageId != null
                  ? BlocProvider(
                      key: ValueKey(messageId),
                      create: (context) => MessageCubit(
                        userCubit: context.read(),
                        messageId: messageId,
                      ),
                      child: const ConversationTile(),
                    )
                  : const SizedBox.shrink();
            },
            findChildIndexCallback: (key) {
              final messageKey = key as ValueKey<UiConversationMessageId>;
              final messageId = messageKey.value;
              return context
                  .read<ConversationDetailsCubit>()
                  .revOffsetFromMessageId(messageId);
            },
            childCount: messagesCount,
          ),
        ),
      ),
    );
  }
}
