// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:provider/provider.dart';
import 'conversation_content/conversation_content.dart';
import 'message_composer.dart';

class ConversationPane extends StatefulWidget {
  const ConversationPane({super.key});

  @override
  State<ConversationPane> createState() => _ConversationPaneState();
}

class _ConversationPaneState extends State<ConversationPane> {
  UiConversationDetails? _currentConversation;
  late final StreamSubscription<UiConversationDetails> _listener;

  @override
  void initState() {
    super.initState();
    final coreClient = context.coreClient;
    _currentConversation = coreClient.currentConversation;
    _listener = coreClient.onConversationSwitch.listen((conversation) {
      setState(() {
        _currentConversation = conversation;
      });
    });
  }

  @override
  void dispose() {
    _listener.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ConversationMessages(currentConversation: _currentConversation);
  }
}

class ConversationMessages extends StatelessWidget {
  const ConversationMessages({
    super.key,
    required UiConversationDetails? currentConversation,
  }) : _currentConversation = currentConversation;

  final UiConversationDetails? _currentConversation;

  @override
  Widget build(BuildContext context) {
    final conversationId =
        context.select((NavigationCubit cubit) => cubit.state.conversationId);
    // only use current conversation if the navigation actually points to it
    final currentConversation =
        conversationId != null ? _currentConversation : null;

    return Scaffold(
      body: Stack(children: <Widget>[
        Column(
          children: [
            ConversationContent(
              conversation: _currentConversation,
            ),
            const MessageComposer(),
          ],
        ),
        Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: AppBar(
            title: Text(_currentConversation?.title ?? ""),
            backgroundColor: Colors.white,
            forceMaterialTransparency: true,
            actions: [
              // Conversation details
              currentConversation != null
                  ? _detailsButton(context)
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
