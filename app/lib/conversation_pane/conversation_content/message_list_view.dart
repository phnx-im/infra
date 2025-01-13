// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/conversation_pane/conversation_details/conversation_details_cubit.dart';
import 'package:prototype/core/api/message_cubit.dart';
import 'package:prototype/core/api/types.dart';
import 'package:visibility_detector/visibility_detector.dart';

import 'conversation_tile.dart';
import 'message_cubit.dart';
import 'message_list_cubit.dart';

class MessageListContainer extends StatelessWidget {
  const MessageListContainer({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.conversation?.id,
    );

    if (conversationId == null) {
      return const SizedBox.shrink();
    }

    return BlocProvider<MessageListCubit>(
      create: (context) => MessageListCubit(
        userCubit: context.read(),
        conversationId: conversationId,
      ),
      child: MessageListView(),
    );
  }
}

class MessageListView extends StatelessWidget {
  const MessageListView({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.select((MessageListCubit cubit) => cubit.state);

    return Expanded(
      child: SelectionArea(
        child: ListView.custom(
          physics: _scrollPhysics,
          reverse: true,
          childrenDelegate: SliverChildBuilderDelegate(
            (context, reverseIndex) {
              final index = state.loadedMessagesCount - reverseIndex - 1;
              final message = state.messageAt(index);
              return message != null
                  ? BlocProvider(
                      key: ValueKey(message.id),
                      create: (context) {
                        return MessageCubit(
                          userCubit: context.read(),
                          messageId: message.id,
                          initialState: MessageState(message: message),
                        );
                      },
                      child: _VisibilityConversationTile(
                        messageId: message.id,
                        timestamp: DateTime.parse(message.timestamp),
                      ),
                    )
                  : const SizedBox.shrink();
            },
            findChildIndexCallback: (key) {
              final messageKey = key as ValueKey<UiConversationMessageId>;
              final messageId = messageKey.value;
              final index = state.messageIdIndex(messageId);
              // reverse index
              return index != null
                  ? state.loadedMessagesCount - index - 1
                  : null;
            },
            childCount: state.loadedMessagesCount,
          ),
        ),
      ),
    );
  }
}

class _VisibilityConversationTile extends StatelessWidget {
  const _VisibilityConversationTile({
    required this.messageId,
    required this.timestamp,
  });

  final UiConversationMessageId messageId;
  final DateTime timestamp;

  @override
  Widget build(BuildContext context) {
    return VisibilityDetector(
      key: ValueKey(_VisibilityKeyValue(messageId)),
      child: const ConversationTile(),
      onVisibilityChanged: (visibilityInfo) {
        if (visibilityInfo.visibleFraction > 0) {
          context.read<ConversationDetailsCubit>().markAsRead(
                untilMessageId: messageId,
                untilTimestamp: timestamp,
              );
        }
      },
    );
  }
}

class _VisibilityKeyValue {
  const _VisibilityKeyValue(this.id);
  final UiConversationMessageId id;
}

final ScrollPhysics _scrollPhysics =
    (Platform.isAndroid || Platform.isWindows || Platform.isLinux)
        ? const ClampingScrollPhysics()
        : const BouncingScrollPhysics()
            .applyTo(const AlwaysScrollableScrollPhysics());
