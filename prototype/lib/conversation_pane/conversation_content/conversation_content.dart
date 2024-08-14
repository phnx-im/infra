// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:collection';
import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/conversation_content/conversation_tile.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:flutter/scheduler.dart';
import 'package:collection/collection.dart';

class ConversationContent extends StatefulWidget {
  final UiConversation? conversation;

  const ConversationContent({super.key, required this.conversation});

  @override
  State<ConversationContent> createState() => _ConversationContentState();
}

class _ConversationContentState extends State<ConversationContent> {
  final ScrollController _scrollController = ScrollController();
  final HashMap<int, GlobalKey> _tileKeys = HashMap();
  Timer? _debounceTimer;
  DateTime? lastRead;
  List<UiConversationMessage> _messages = [];
  UiConversation? _currentConversation;
  StreamSubscription<UiConversation>? _conversationListener;
  StreamSubscription<UiConversationMessage>? _messageListener;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);

    _conversationListener =
        coreClient.onConversationSwitch.listen(conversationListener);
    _messageListener = coreClient.onMessageUpdate.listen(messageListener);
    _currentConversation = widget.conversation;

    if (_currentConversation != null) {
      // Updates the messages and scrolls to the end of the conversation.
      updateMessages().then((_) => scrollToEnd());
    }

    // Call _processVisibleMessages once initially after the first frame is built
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _processVisibleMessages();
    });
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
          final messageTimestamp = _messages[index].timestamp;
          lastRead ??= messageTimestamp;

          if (lastRead != null && messageTimestamp.isAfter(lastRead!)) {
            lastRead = messageTimestamp;
            _onMessageVisible(messageTimestamp);
          }
        }
      }
    }
  }

  void _onMessageVisible(DateTime timestamp) {
    if (_currentConversation != null) {
      coreClient.user.markMessagesAsReadDebounced(
        conversationId: _currentConversation!.id,
        timestamp: timestamp,
      );
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _processVisibleMessages();
  }

  Future<void> updateMessages() async {
    if (_currentConversation != null) {
      final messages = await coreClient.user
          .getMessages(conversationId: _currentConversation!.id, lastN: 100);
      setState(() {
        print("Number of messages: ${messages.length}");
        _messages = messages;
      });
    }
  }

  void conversationListener(UiConversation conversation) async {
    _currentConversation = conversation;
    _messages = [];
    await updateMessages();
  }

  void messageListener(UiConversationMessage cm) {
    if (cm.conversationId.bytes.equals(_currentConversation!.id.bytes)) {
      setState(() {
        _messages.add(cm);
        scrollToEnd();
      });
    } else {
      print('A message for another group was received');
    }
  }

  // Smooth scrolling to the end of the conversation
  void scrollToEnd() {
    setState(() {
      SchedulerBinding.instance.addPostFrameCallback((_) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeInOut,
        );
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
          physics: const BouncingScrollPhysics(),
          itemBuilder: (BuildContext context, int index) {
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
