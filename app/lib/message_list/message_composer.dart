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

class _MessageComposerState extends State<MessageComposer>
    with WidgetsBindingObserver {
  final TextEditingController _controller = CustomTextEditingController();
  final _focusNode = FocusNode();
  bool _keyboardVisible = false;

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
    WidgetsBinding.instance.addObserver(this);
    _focusNode.onKeyEvent =
        (focusNode, event) => _onKeyEvent(context.read(), focusNode, event);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    final view = View.of(context);
    final bottomInset = view.viewInsets.bottom;
    final newValue = bottomInset > 0.0;

    if (_keyboardVisible != newValue) {
      setState(() {
        _keyboardVisible = newValue;
      });
    }
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
    final conversationTitle = context.select(
      (ConversationDetailsCubit cubit) =>
          cubit.state.conversation?.attributes.title,
    );

    if (conversationTitle == null) {
      return const SizedBox.shrink();
    }

    return AnimatedContainer(
      duration: const Duration(milliseconds: 1000),
      child: Container(
        color: Colors.white.withValues(alpha: 0.9),
        padding: EdgeInsets.only(
          top: Spacings.xs,
          bottom:
              isSmallScreen(context) && !_keyboardVisible
                  ? Spacings.m
                  : Spacings.xs,
          left: Spacings.xs,
          right: Spacings.xs,
        ),
        child: Row(
          children: [
            Expanded(
              child: Container(
                decoration: BoxDecoration(
                  color: convPaneBackgroundColor.withValues(alpha: 0.9),
                  borderRadius: BorderRadius.circular(Spacings.m),
                ),
                padding: const EdgeInsets.only(
                  left: Spacings.xs,
                  right: Spacings.xs,
                ),
                child: _MessageInput(
                  focusNode: _focusNode,
                  controller: _controller,
                  conversationTitle: conversationTitle,
                ),
              ),
            ),
            if (isSmallScreen(context))
              Container(
                width: 50,
                height: 50,
                margin: const EdgeInsets.only(left: Spacings.xs),
                decoration: BoxDecoration(
                  color: convPaneBackgroundColor.withValues(alpha: 0.9),
                  borderRadius: BorderRadius.circular(Spacings.m),
                ),
                child: IconButton(
                  icon: const Icon(Icons.send),
                  color: colorDMB,
                  hoverColor: const Color(0x00FFFFFF),
                  onPressed: () {
                    _submitMessage(context.read());
                  },
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _MessageInput extends StatelessWidget {
  const _MessageInput({
    required FocusNode focusNode,
    required TextEditingController controller,
    required this.conversationTitle,
  }) : _focusNode = focusNode,
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
        hintStyle: Theme.of(
          context,
        ).textTheme.bodyMedium?.copyWith(color: colorDMB),
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
