// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:collection';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/conversation_content/conversation_tile.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';

class ConversationContent extends StatefulWidget {
  final UiConversationDetails? conversation;

  const ConversationContent({super.key, required this.conversation});

  @override
  State<ConversationContent> createState() => _ConversationContentState();
}

class _ConversationContentState extends State<ConversationContent> {
  final ScrollController _scrollController =
      TrackingScrollController(keepScrollOffset: true);
  final ScrollPhysics _scrollPhysics =
      (Platform.isAndroid || Platform.isWindows)
          ? const ClampingScrollPhysics()
          : const BouncingScrollPhysics()
              .applyTo(const AlwaysScrollableScrollPhysics());

  final HashMap<int, GlobalKey> _tileKeys = HashMap();
  Timer? _debounceTimer;
  List<UiConversationMessage> _messages = [];
  UiConversationDetails? _currentConversation;
  StreamSubscription<UiConversationDetails>? _conversationListener;
  StreamSubscription<UiConversationMessage>? _messageListener;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);

    final coreClient = context.coreClient;
    _conversationListener =
        coreClient.onConversationSwitch.listen(conversationListener);
    _messageListener = coreClient.onMessageUpdate.listen(messageListener);
    _currentConversation = widget.conversation;

    if (_currentConversation != null) {
      // Updates the messages and scrolls to the end of the conversation.
      updateMessages().then((_) => scrollToEnd());
    }
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _debounceTimer?.cancel();
    _conversationListener?.cancel();
    _messageListener?.cancel();
    super.dispose();
  }

  void _onScroll() {
    // Cancel the previous timer if still active
    if (_debounceTimer?.isActive ?? false) {
      _debounceTimer?.cancel();
    }

    // Start a new timer to debounce the scroll events
    _debounceTimer =
        Timer(const Duration(milliseconds: 100), _processVisibleMessages);
  }

  void _processVisibleMessages() {
    if (!_scrollController.hasClients) {
      return;
    }

    final viewportHeight = _scrollController.position.viewportDimension;

    // Iterate over the key value pairs
    for (final entry in _tileKeys.entries) {
      final key = entry.value;
      final index = entry.key;
      final renderObject = key.currentContext?.findRenderObject();
      if (renderObject is RenderBox) {
        final position = renderObject.localToGlobal(Offset.zero);
        final size = renderObject.size;

        final topEdgeVisible = position.dy >= 0;
        final bottomEdgeVisible = position.dy + size.height <= viewportHeight;

        if (topEdgeVisible && bottomEdgeVisible) {
          _onMessageVisible(_messages[index].timestamp);
        }
      }
    }
  }

  void _onMessageVisible(String timestamp) {
    if (_currentConversation != null) {
      context.coreClient.user.markMessagesAsReadDebounced(
          conversationId: _currentConversation!.id, timestamp: timestamp);
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _processVisibleMessages();
  }

  Future<void> updateMessages() async {
    if (_currentConversation != null) {
      final messages = await context.coreClient.user
          .getMessages(conversationId: _currentConversation!.id, lastN: 50);
      setState(() {
        print("Number of messages: ${messages.length}");
        _messages = messages;
      });
    }
  }

  void conversationListener(UiConversationDetails conversation) async {
    _currentConversation = conversation;
    _messages = [];
    await updateMessages();
    scrollToEnd();
  }

  void messageListener(UiConversationMessage cm) {
    if (cm.conversationId == _currentConversation!.id) {
      setState(() {
        updateMessages().then((_) {
          scrollToEnd(animated: true);
        });
      });
    } else {
      print('A message for another group was received');
    }
  }

  // Smooth scrolling to the end of the conversation
  // with an optional parameter to enable/disable animation
  void scrollToEnd({
    bool animated = false,
  }) {
    setState(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final extent = _scrollController.position.minScrollExtent;
        if (animated) {
          _scrollController.animateTo(
            extent,
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeInOut,
          );
        } else {
          _scrollController.jumpTo(extent);
        }
        _processVisibleMessages();
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: SelectionArea(
        child: ListView.builder(
          controller: _scrollController,
          padding: EdgeInsets.only(
            top: kToolbarHeight +
                MediaQuery.of(context)
                    .padding
                    .top, // Use the AppBar's height as padding
            left: 10,
          ),
          itemCount: _messages.length,
          physics: _scrollPhysics,
          reverse: true,
          itemBuilder: (BuildContext context, int index) {
            // Reverse the index to display the messages in the correct order
            index = _messages.length - 1 - index;
            final key = GlobalKey();
            _tileKeys[index] = key;
            final message = _messages[index];
            return ConversationTile(key: key, message: message);
          },
        ),
      ),
    );
  }
}
