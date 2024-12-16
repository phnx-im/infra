// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/conversation_list_pane/conversation_list_cubit.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'conversation_content/conversation_content.dart';
import 'message_composer.dart';

class ConversationPaneContainer extends StatelessWidget {
  const ConversationPaneContainer({super.key});

  @override
  Widget build(BuildContext context) {
    // TODO: this is a temporary solution until ConversationMessages gets its own cubit
    return BlocProvider<ConversationListCubit>(
      create: (context) => ConversationListCubit(userCubit: context.read()),
      child: ConversationPane(),
    );
  }
}

class ConversationPane extends StatelessWidget {
  const ConversationPane({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId =
        context.select((NavigationCubit cubit) => cubit.state.conversationId);
    final currentConversation = context.select((ConversationListCubit cubit) =>
        conversationId != null
            ? cubit.state.conversations.firstWhereOrNull(
                (conversation) => conversation.id == conversationId)
            : null);

    return Scaffold(
      body: Stack(children: <Widget>[
        Column(
          children: [
            ConversationContent(conversation: currentConversation),
            const MessageComposer(),
          ],
        ),
        Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: AppBar(
            title: Text(currentConversation?.title ?? ""),
            backgroundColor: Colors.white,
            forceMaterialTransparency: true,
            actions: [
              // Conversation details
              currentConversation != null
                  ? _detailsButton(context)
                  : Container(),
            ],
            leading: context.responsiveScreenType == ResponsiveScreenType.mobile
                ? const _BackButton()
                : null,
            elevation: 0,
            // Applying blur effect
            flexibleSpace: FrostedGlass(
                color: Colors.white,
                height: kToolbarHeight + MediaQuery.of(context).padding.top),
          ),
        ),
      ]),
    );
  }

  IconButton _detailsButton(BuildContext context) {
    return IconButton(
      icon: const Icon(
        Icons.more_horiz,
        size: 28,
      ),
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
