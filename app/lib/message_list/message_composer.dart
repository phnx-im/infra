// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:collection';

import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/theme/theme.dart';
import 'package:provider/provider.dart';

import 'message_renderer.dart';

class MessageComposer extends StatefulWidget {
  const MessageComposer({super.key});

  @override
  State<MessageComposer> createState() => _MessageComposerState();
}

class _MessageComposerState extends State<MessageComposer> {
  final TextEditingController _controller = _CustomTextEditingController();
  final _focusNode = FocusNode();
  final _keywords = [
    "@Alice",
    "@Bob",
    "@Carol",
    "@Dave",
    "@Eve",
  ];

  // Override constructor
  _MessageComposerState() {
    (_controller as _CustomTextEditingController).keywords = _keywords;
  }

  // Key events
  KeyEventResult _onKeyEvent(
    ConversationDetailsCubit conversationDetailCubit,
    FocusNode node,
    KeyEvent evt,
  ) {
    final keyId = evt.logicalKey.keyId;
    if (!HardwareKeyboard.instance.isShiftPressed &&
        !HardwareKeyboard.instance.isAltPressed &&
        !HardwareKeyboard.instance.isMetaPressed &&
        !HardwareKeyboard.instance.isControlPressed &&
        (evt.logicalKey.keyLabel == "Enter") &&
        (evt is KeyDownEvent)) {
      _submitMessage(conversationDetailCubit);
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
    _focusNode.onKeyEvent =
        (focusNode, event) => _onKeyEvent(context.read(), focusNode, event);
  }

  void _submitMessage(ConversationDetailsCubit conversationDetailsCubit) async {
    final messageText = _controller.text.trim();
    if (messageText.isEmpty) {
      return;
    }

    // FIXME: Handle errors
    conversationDetailsCubit.sendMessage(messageText);

    setState(() {
      _controller.clear();
      _focusNode.requestFocus();
    });
  }

  @override
  Widget build(BuildContext context) {
    final conversationTitle = context.select((ConversationDetailsCubit cubit) =>
        cubit.state.conversation?.attributes.title);

    if (conversationTitle == null) {
      return const SizedBox.shrink();
    }

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
                child: _MessageInput(
                  focusNode: _focusNode,
                  controller: _controller,
                  conversationTitle: conversationTitle,
                ),
              ),
              if (isSmallScreen(context))
                Container(
                  width: 40,
                  margin: const EdgeInsets.all(10),
                  child: IconButton(
                    icon: const Icon(Icons.send),
                    color: colorDMB,
                    hoverColor: const Color(0x00FFFFFF),
                    onPressed: () {
                      _submitMessage(context.read());
                    },
                  ),
                )
            ],
          ),
        ],
      ),
    );
  }
}

class _MessageInput extends StatelessWidget {
  const _MessageInput({
    required FocusNode focusNode,
    required TextEditingController controller,
    required this.conversationTitle,
  })  : _focusNode = focusNode,
        _controller = controller;

  final FocusNode _focusNode;
  final TextEditingController _controller;
  final String? conversationTitle;

  @override
  Widget build(BuildContext context) {
    final smallScreen = isSmallScreen(context);

    final style = Theme.of(context).textTheme.bodyLarge!.merge(
        smallScreen ? VariableFontWeight.medium : VariableFontWeight.normal);

    return TextField(
      focusNode: _focusNode,
      style: style,
      controller: _controller,
      minLines: 1,
      maxLines: 10,
      decoration: InputDecoration(
        hintText: "Message $conversationTitle",
      ).copyWith(filled: false),
      textInputAction:
          smallScreen ? TextInputAction.send : TextInputAction.newline,
      onEditingComplete: () => _focusNode.requestFocus(),
      keyboardType: TextInputType.multiline,
      textCapitalization: TextCapitalization.sentences,
    );
  }
}

enum Direction { right, left }

class _CustomTextEditingController extends TextEditingController {
  _CustomTextEditingController();

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
