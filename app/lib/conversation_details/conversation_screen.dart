// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/message_list/message_list.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';

import 'conversation_details_cubit.dart';

class ConversationScreen extends StatelessWidget {
  const ConversationScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    if (conversationId == null) {
      return const _EmptyConversationPane();
    }

    return MultiBlocProvider(
      providers: [
        BlocProvider(
          // rebuilds the cubit when a different conversation is selected
          key: ValueKey("message-list-cubit-$conversationId"),
          create:
              (context) => MessageListCubit(
                userCubit: context.read<UserCubit>(),
                conversationId: conversationId,
              ),
        ),
      ],
      child: const ConversationScreenView(),
    );
  }
}

class _EmptyConversationPane extends StatelessWidget {
  const _EmptyConversationPane();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Text(
          style: Theme.of(
            context,
          ).textTheme.labelMedium?.copyWith(color: colorDMB),
          "Select a chat to start messaging",
        ),
      ),
    );
  }
}

class ConversationScreenView extends StatelessWidget {
  const ConversationScreenView({
    super.key,
    this.createMessageCubit = MessageCubit.new,
  });

  final MessageCubitCreate createMessageCubit;

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    final conversationTitle = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.conversation?.title,
    );

    if (conversationId == null) {
      return const _EmptyConversationPane();
    }

    return Scaffold(
      body: Stack(
        children: <Widget>[
          Column(
            children: [
              Expanded(
                child: MessageListView(createMessageCubit: createMessageCubit),
              ),
              const MessageComposer(),
            ],
          ),
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            child: AppBar(
              title: Text(conversationTitle ?? ""),
              backgroundColor: Colors.white,
              forceMaterialTransparency: true,
              actions: [
                // Conversation details
                conversationTitle != null
                    ? const _DetailsButton()
                    : const SizedBox.shrink(),
              ],
              leading:
                  context.responsiveScreenType == ResponsiveScreenType.mobile
                      ? const _BackButton()
                      : null,
              elevation: 0,
              // Applying blur effect
              flexibleSpace: FrostedGlass(
                color: Colors.white,
                height: kToolbarHeight + MediaQuery.of(context).padding.top,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _DetailsButton extends StatelessWidget {
  const _DetailsButton();

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.more_horiz, size: 28),
      color: Colors.black,
      padding: const EdgeInsets.symmetric(horizontal: 20),
      hoverColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      onPressed: () {
        context.read<NavigationCubit>().openConversationDetails();
      },
    );
  }
}

class _BackButton extends StatelessWidget {
  const _BackButton();

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.arrow_back),
      color: Colors.black,
      hoverColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      onPressed: () {
        context.read<NavigationCubit>().closeConversation();
      },
    );
  }
}
