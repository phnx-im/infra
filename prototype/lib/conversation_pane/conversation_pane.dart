// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:applogic/applogic.dart';
import 'package:prototype/conversation_pane/conversation_details/conversation_details.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/main.dart';
import 'package:prototype/messenger_view.dart';
import 'package:prototype/styles.dart';
import 'conversation_content/conversation_content.dart';
import 'message_composer.dart';

class ConversationPane extends StatefulWidget {
  final GlobalKey<NavigatorState> navigatorKey;

  const ConversationPane(this.navigatorKey, {super.key});

  @override
  State<ConversationPane> createState() => _ConversationPaneState();
}

class _ConversationPaneState extends State<ConversationPane> {
  UiConversation? _currentConversation;
  late StreamSubscription<UiConversation> _listener;

  @override
  void initState() {
    super.initState();
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
    return Navigator(
      key: widget.navigatorKey,
      onGenerateInitialRoutes: (navigator, initialRoute) {
        return [
          MaterialPageRoute(
            builder: (context) => ConversationMessages(
              currentConversation: _currentConversation,
              context: context,
            ),
          ),
        ];
      },
    );
  }

/*   @override
  Widget build(BuildContext context) {
    return ConversationMessages(
      currentConversation: _currentConversation,
      context: context,
    );
  } */
}

class ConversationMessages extends StatelessWidget {
  const ConversationMessages({
    super.key,
    required UiConversation? currentConversation,
    required this.context,
  }) : _currentConversation = currentConversation;

  final UiConversation? _currentConversation;
  final BuildContext context;

  @override
  Widget build(BuildContext context) {
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
            title: Text(
              _currentConversation?.conversationType.when(
                      unconfirmedConnection: (e) => '⏳ $e',
                      connection: (e) => e,
                      group: () => _currentConversation?.attributes.title) ??
                  "",
            ),
            backgroundColor: Colors.white,
            forceMaterialTransparency: true,
            actions: [
              // Conversation details
              _currentConversation != null
                  ? IconButton(
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
                    )
                  : Container(),
            ],
            leading: isSmallScreen(context)
                ? IconButton(
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
                  )
                : null,
            elevation: 0,
            // Applying blur effect
            flexibleSpace: ClipRect(
              child: BackdropFilter(
                filter: ImageFilter.blur(sigmaX: 5, sigmaY: 5),
                child: Container(
                  width: MediaQuery.of(context).size.width,
                  height: kToolbarHeight + MediaQuery.of(context).padding.top,
                  color: Colors.transparent,
                ),
              ),
            ),
          ),
        ),
      ]),
    );
  }
}
