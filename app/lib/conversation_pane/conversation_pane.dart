// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'conversation_content/message_list_view.dart';
import 'conversation_cubit.dart';
import 'message_composer.dart';

class ConversationPaneContainer extends StatelessWidget {
  const ConversationPaneContainer({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId =
        context.select((NavigationCubit cubit) => cubit.state.conversationId);
    if (conversationId == null) {
      throw StateError("an active conversation is obligatory");
    }

    return BlocProvider(
      create: (context) => ConversationCubit(
        userCubit: context.read(),
        conversationId: conversationId,
      ),
      child: const ConversationPane(),
    );
  }
}

class ConversationPane extends StatelessWidget {
  const ConversationPane({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationTitle = context
        .select((ConversationCubit cubit) => cubit.state.conversation?.title);

    return Scaffold(
      body: Stack(children: <Widget>[
        Column(
          children: [
            const MessageListView(),
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
}

class _DetailsButton extends StatelessWidget {
  const _DetailsButton();

  @override
  Widget build(BuildContext context) {
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
