// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:prototype/core/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/conversation_pane/message_renderer.dart';
import 'package:prototype/styles.dart';

enum Direction { right, left }

class CustomTextEditingController extends TextEditingController {
  CustomTextEditingController({super.text});

  List<String> _keywords = [];

  // Setter for keywords
  set keywords(List<String> keywords) {
    _keywords = keywords;
    notifyListeners();
  }

  // Getter for keywords
  List<String> get keywords => _keywords;

  @override
  TextSpan buildTextSpan(
      {required BuildContext context,
      TextStyle? style,
      required bool withComposing}) {
    return buildTextSpanFromText(_keywords, text, style, HostWidget.textField);
  }
}

class MessageComposer extends StatefulWidget {
  const MessageComposer({super.key});

  @override
  State<MessageComposer> createState() => _MessageComposerState();
}

class _MessageComposerState extends State<MessageComposer> {
  final TextEditingController _controller = CustomTextEditingController();
  final _focusNode = FocusNode();
  final _keywords = [
    "@Alice",
    "@Bob",
    "@Carol",
    "@Dave",
    "@Eve",
  ];

  UiConversation? _currentConversation;
  late StreamSubscription<UiConversation> _listener;

  HashMap<ConversationIdBytes, String> drafts = HashMap();

  // Override constructor
  _MessageComposerState() {
    (_controller as CustomTextEditingController).keywords = _keywords;
  }

  // Key events
  KeyEventResult onKeyEvent(FocusNode node, KeyEvent evt) {
    final keyId = evt.logicalKey.keyId;
    if (!HardwareKeyboard.instance.isShiftPressed &&
        !HardwareKeyboard.instance.isAltPressed &&
        !HardwareKeyboard.instance.isMetaPressed &&
        !HardwareKeyboard.instance.isControlPressed &&
        (evt.logicalKey.keyLabel == "Enter") &&
        (evt is KeyDownEvent)) {
      submitMessage();
      return KeyEventResult.handled;
      // Arrow keys
    } else if (keyId == LogicalKeyboardKey.arrowLeft.keyId ||
        keyId == LogicalKeyboardKey.arrowRight.keyId) {
      final direction = keyId == LogicalKeyboardKey.arrowLeft.keyId
          ? Direction.left
          : Direction.right;
      final isSelection =
          _controller.selection.base != _controller.selection.extent;
      final position = isSelection
          ? _controller.selection.extentOffset
          : _controller.selection.baseOffset;

      final matches = _keywords
          .map((keyword) => RegExp(keyword, caseSensitive: false)
              .allMatches(_controller.text))
          .expand((element) => element)
          .toList();

      final match = matches
          .where((element) =>
              // Check if the cursor is in the middle of a match
              element.start < position && element.end > position)
          .singleOrNull;
      // Then move the cursor to the start or the end of the match, depending
      // on the direction
      if (match != null) {
        final target = direction == Direction.left ? match.start : match.end;
        _controller.selection = TextSelection(
            baseOffset: isSelection ? _controller.selection.baseOffset : target,
            extentOffset: target);
      }
      return KeyEventResult.ignored;
    } else {
      return KeyEventResult.ignored;
    }
  }

  @override
  void initState() {
    super.initState();
    _listener = coreClient.onConversationSwitch.listen(conversationListener);
    _currentConversation = coreClient.currentConversation;
    _focusNode.onKeyEvent = onKeyEvent;
  }

  @override
  void dispose() {
    _listener.cancel();
    super.dispose();
  }

  void conversationListener(UiConversation cc) {
    // Store draft for the current conversation
    if (_currentConversation != null) {
      drafts.addEntries([MapEntry(_currentConversation!.id, _controller.text)]);
    }
    setState(() {
      _currentConversation = cc;
      _controller.clear();
      // Look up previous drafts
      String? draft = drafts.remove(_currentConversation!.id);
      if (draft != null) {
        _controller.text = draft;
      }
    });
  }

  void submitMessage() async {
    var message = utf8.encode(_controller.text.trim());
    if (message.isEmpty) {
      return;
    }

    await coreClient.sendMessage(
        coreClient.currentConversation!.id, _controller.text.trim());
    _controller.clear();
    _focusNode.requestFocus();
  }

  String? hintText() {
    if (_currentConversation != null) {
      final name = _currentConversation!.conversationType.when(
          unconfirmedConnection: (e) => e,
          connection: (e) => e,
          group: () => _currentConversation!.attributes.title);
      return _currentConversation != null ? 'Message $name' : '';
    } else {
      return "";
    }
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedContainer(
      duration: const Duration(milliseconds: 1000),
      padding: const EdgeInsets.only(
        left: 10,
        top: 0,
        right: 10,
        bottom: 5,
      ),
      child: Column(
        children: [
          if (_currentConversation != null)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Container(
                height: 1.5,
                color: colorGreyLight,
              ),
            ),
          Row(
            children: [
              Expanded(
                child: TextField(
                  enabled: _currentConversation != null,
                  focusNode: _focusNode,
                  style: messageTextStyle(context),
                  controller: _controller,
                  minLines: 1,
                  maxLines: 10,
                  decoration: messageComposerInputDecoration(context)
                      .copyWith(hintText: hintText()),
                  textInputAction: isSmallScreen(context)
                      ? TextInputAction.send
                      : TextInputAction.newline,
                  onEditingComplete: () => _focusNode.requestFocus(),
                ),
              ),
              isSmallScreen(context)
                  ? Container(
                      width: 40,
                      margin: const EdgeInsets.all(10),
                      child: IconButton(
                        icon: const Icon(Icons.send),
                        color: colorDMB,
                        hoverColor: const Color(0x00FFFFFF),
                        onPressed: () {
                          submitMessage();
                        },
                      ),
                    )
                  : Container(),
            ],
          ),
        ],
      ),
    );
  }
}
