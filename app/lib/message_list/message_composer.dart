// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
  final TextEditingController _controller = CustomTextEditingController();
  final _focusNode = FocusNode();

  // Key events
  KeyEventResult _onKeyEvent(
    ConversationDetailsCubit conversationDetailCubit,
    FocusNode node,
    KeyEvent evt,
  ) {
    if (!HardwareKeyboard.instance.isShiftPressed &&
        !HardwareKeyboard.instance.isAltPressed &&
        !HardwareKeyboard.instance.isMetaPressed &&
        !HardwareKeyboard.instance.isControlPressed &&
        (evt.logicalKey.keyLabel == "Enter") &&
        (evt is KeyDownEvent)) {
      _submitMessage(conversationDetailCubit);
      return KeyEventResult.handled;
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

    return TextField(
      focusNode: _focusNode,
      style: messageTextStyle(context, false),
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
