import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/conversation_pane/conversation_cubit.dart';

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
      (ConversationCubit cubit) => cubit.state.messagesCount,
    );

    return Expanded(
      child: SelectionArea(
        child: ListView.builder(
          padding: EdgeInsets.only(
            top: kToolbarHeight +
                MediaQuery.of(context)
                    .padding
                    .top, // Use the AppBar's height as padding
            left: 10,
          ),
          itemCount: messagesCount,
          physics: _scrollPhysics,
          reverse: true,
          itemBuilder: (BuildContext context, int index) {
            final messageId =
                context.read<ConversationCubit>().messageIdFromRevOffset(index);
            return BlocProvider(
              create: (context) => MessageCubit(
                userCubit: context.read(),
                messageId: messageId,
              ),
              child: ConversationTile(key: ValueKey(messageId)),
            );
          },
        ),
      ),
    );
  }
}
