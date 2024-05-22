import 'dart:async';

import 'package:applogic/applogic.dart';
import 'package:flutter/material.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/conversation_pane/conversation_pane.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/messenger_view.dart';
import '../styles.dart';
import 'package:convert/convert.dart';
import 'package:collection/collection.dart';

class ConversationList extends StatefulWidget {
  const ConversationList({super.key});

  @override
  State<ConversationList> createState() => _ConversationListState();
}

class _ConversationListState extends State<ConversationList> {
  late List<UiConversation> _conversations;
  UiConversation? _currentConversation;
  StreamSubscription<ConversationIdBytes>? _conversationListUpdateListener;
  StreamSubscription<UiConversation>? _conversationSwitchListener;
  final ScrollController _scrollController = ScrollController();

  _ConversationListState() {
    _conversations = coreClient.conversationsList;
    _currentConversation = coreClient.currentConversation;
    _conversationListUpdateListener = coreClient.onConversationListUpdate
        .listen(conversationListUpdateListener);
    _conversationSwitchListener =
        coreClient.onConversationSwitch.listen(conversationSwitchListener);
  }

  @override
  void initState() {
    super.initState();
    updateConversationList();
  }

  @override
  void dispose() {
    _conversationListUpdateListener?.cancel();
    _conversationSwitchListener?.cancel();
    super.dispose();
  }

  void conversationSwitchListener(UiConversation cc) {
    if (_currentConversation != null) {
      if (_currentConversation!.id != cc.id) {
        setState(() {
          _currentConversation = cc;
        });
      }
    } else {
      _currentConversation = cc;
    }
  }

  void selectConversation(ConversationIdBytes conversationId) {
    print("Tapped on conversation ${hex.encode(conversationId.bytes)}");
    coreClient.selectConversation(conversationId);
    if (isSmallScreen(context)) {
      pushToNavigator(context, ConversationPane(navigatorKey));
    }
  }

  void conversationListUpdateListener(ConversationIdBytes uuid) async {
    updateConversationList();
  }

  void updateConversationList() async {
    await coreClient.conversations().then((conversations) {
      setState(() {
        if (_currentConversation == null && conversations.isNotEmpty) {
          selectConversation(conversations[0].id);
        }
        _conversations = conversations;
      });
    });
  }

  Color? selectionColor(int index) {
    if (isLargeScreen(context) &&
        _currentConversation != null &&
        _currentConversation!.id.bytes.equals(_conversations[index].id.bytes)) {
      return convPaneFocusColor;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    if (_conversations.isNotEmpty) {
      return Column(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: ListView.builder(
              padding: const EdgeInsets.all(0),
              itemCount: _conversations.length,
              physics: const BouncingScrollPhysics(),
              controller: _scrollController,
              itemBuilder: (BuildContext context, int index) {
                return ListTile(
                  horizontalTitleGap: 0,
                  title: Container(
                    alignment: AlignmentDirectional.topStart,
                    width: 300,
                    padding: const EdgeInsets.all(15),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(5.0),
                      color: selectionColor(index),
                    ),
                    child: Row(
                      children: [
                        UserAvatar(
                          size: 48,
                          image: _conversations[index]
                              .attributes
                              .conversationPictureOption,
                          username: _conversations[index].conversationType.when(
                              unconfirmedConnection: (e) => e,
                              connection: (e) => e,
                              group: () =>
                                  _conversations[index].attributes.title),
                        ),
                        const SizedBox(
                          width: 16,
                        ),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                _conversations[index].conversationType.when(
                                    unconfirmedConnection: (e) => '⏳ $e',
                                    connection: (e) => e,
                                    group: () =>
                                        _conversations[index].attributes.title),
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  color: convListItemTextColor,
                                  fontSize: 14,
                                  fontVariations: variationSemiBold,
                                  letterSpacing: -0.2,
                                ),
                              ),
                              const SizedBox(
                                height: 5,
                              ),
                              Text(
                                _conversations[index].conversationType.when(
                                    unconfirmedConnection: (e) =>
                                        'Pending connection request',
                                    connection: (e) => '1:1 conversation',
                                    group: () => 'Group conversation'),
                                style: const TextStyle(
                                  color: colorDMB,
                                  fontSize: 12,
                                  fontVariations: variationRegular,
                                  letterSpacing: -0.2,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  selected: isConversationSelected(
                      _currentConversation, _conversations[index], context),
                  focusColor: convListItemSelectedColor,
                  onTap: () => selectConversation(_conversations[index].id),
                );
              },
            ),
          ),
          // Show footer only if there are more conversations than can fit on
          // the screen
          (_scrollController.hasClients &&
                  _scrollController.position.maxScrollExtent > 0)
              ? Column(
                  children: [
                    Container(
                      width: 200,
                      height: 1.5,
                      color: colorDMBLight,
                    ),
                  ],
                )
              : Container(),
        ],
      );
    } else {
      return Container(
        alignment: AlignmentDirectional.center,
        child: Text(
          'Create a new connection to get started',
          style: TextStyle(
            fontSize: isLargeScreen(context) ? 14 : 15,
            fontWeight: FontWeight.normal,
            color: Colors.black54,
          ),
        ),
      );
    }
  }
}

bool isConversationSelected(UiConversation? currentConversation,
    UiConversation conversation, BuildContext context) {
  if (isLargeScreen(context) && currentConversation != null) {
    return currentConversation.id.bytes.equals(conversation.id.bytes);
  }
  return false;
}
