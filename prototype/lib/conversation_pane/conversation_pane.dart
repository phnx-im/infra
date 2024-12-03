// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/conversation_details/conversation_details.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/main.dart';
import 'package:prototype/messenger_view.dart';
import 'package:prototype/state_provider.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';
import 'conversation_content/conversation_content.dart';
import 'conversation_cubit.dart';
import 'message_composer.dart';

class ConversationPane extends StatelessWidget {
  const ConversationPane({super.key});

  @override
  Widget build(BuildContext context) {
    return Navigator(
      key: navigatorKey,
      onGenerateInitialRoutes: (navigator, initialRoute) {
        return [
          MaterialPageRoute(
            builder: (context) => StateProvider<CurrentConversationCubit>(
                create: (context) => CurrentConversationCubit(
                      // TODO: This should be injected via a Provider and not singleton
                      coreClient: coreClient,
                    ),
                child: const ConversationMessages()),
          ),
        ];
      },
    );
  }
}

class ConversationMessages extends StatelessWidget {
  const ConversationMessages({super.key});

  @override
  Widget build(BuildContext context) {
    final currentConversation = context.watch<CurrentConversationCubit>().state;

    return Scaffold(
      body: Stack(children: <Widget>[
        Column(
          children: [
            ConversationContent(
              // TODO: this should be passed via a provider
              conversation: currentConversation,
            ),
            const MessageComposer(),
          ],
        ),
        Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: AppBar(
            title: Text(
              currentConversation?.conversationType.when(
                      unconfirmedConnection: (e) => '⏳ $e',
                      connection: (e) => e,
                      group: () => currentConversation.attributes.title) ??
                  "",
            ),
            backgroundColor: Colors.white,
            forceMaterialTransparency: true,
            actions: [
              // Conversation details
              currentConversation != null
                  ? const _DetailsButton()
                  : const SizedBox.shrink(),
            ],
            leading: isSmallScreen(context) ? const _BackButton() : null,
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
      onPressed: () async {
        if (appNavigator.currentState != null) {
          appNavigator.currentState!.maybePop();
        }
      },
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
        pushToNavigator(context, const ConversationDetails());
      },
    );
  }
}
