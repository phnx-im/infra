// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:logging/logging.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart' show AppLocalizations;
import 'package:prototype/main.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/util/debouncer.dart';
import 'package:provider/provider.dart';

import 'message_renderer.dart';

final _log = Logger("MessageComposer");

class MessageComposer extends StatefulWidget {
  const MessageComposer({super.key});

  @override
  State<MessageComposer> createState() => _MessageComposerState();
}

class _MessageComposerState extends State<MessageComposer>
    with WidgetsBindingObserver {
  final TextEditingController _inputController = CustomTextEditingController();
  final Debouncer _storeDraftDebouncer = Debouncer(
    delay: const Duration(milliseconds: 500),
  );
  StreamSubscription<ConversationDetailsState>? _draftLoadingSubscription;
  final _focusNode = FocusNode();
  late ConversationDetailsCubit _conversationDetailsCubit;
  bool _keyboardVisible = false;
  bool _inputIsEmpty = true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _focusNode.onKeyEvent =
        (focusNode, event) => _onKeyEvent(context.read(), focusNode, event);
    _inputController.addListener(_onTextChanged);

    _conversationDetailsCubit = context.read<ConversationDetailsCubit>();

    // Propagate draft changes to the text field.
    // In particular, this sets the draft message on initial load, if any.

    _draftLoadingSubscription = _conversationDetailsCubit.stream.listen((
      state,
    ) {
      if (state.conversation != null) {
        // state is fully loaded
        if (state.conversation?.draft case final draft?) {
          // We have a draft
          // Ignore user drafts, those were input here and just reflect the change state.

          switch (draft.source) {
            case UiMessageDraftSource.system:
              // If input controller is not empty, then the user already typed something,
              // and we don't want to overwrite it.
              if (_inputController.text.isEmpty) {
                _inputController.text = draft.message;
              }
              break;
            case UiMessageDraftSource.user:
              // Ingore user drafts; they just reflect the past state of the input controller.
              break;
          }
        }
      }
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _storeDraftDebouncer.dispose();

    _conversationDetailsCubit.storeDraft(draftMessage: _inputController.text);
    _inputController.dispose();

    _draftLoadingSubscription?.cancel();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    final view = View.of(context);
    final bottomInset = view.viewInsets.bottom;
    final keyboardVisible = bottomInset > 0.0;

    if (_keyboardVisible != keyboardVisible) {
      setState(() {
        _keyboardVisible = keyboardVisible;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final (conversationTitle, editingId) = context.select(
      (ConversationDetailsCubit cubit) => (
        cubit.state.conversation?.title,
        cubit.state.conversation?.draft?.editingId,
      ),
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
          crossAxisAlignment: CrossAxisAlignment.end,
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
                  controller: _inputController,
                  conversationTitle: conversationTitle,
                  isEditing: editingId != null,
                ),
              ),
            ),
            if (editingId != null)
              Container(
                width: 50,
                height: 50,
                margin: const EdgeInsets.only(left: Spacings.xs),
                decoration: BoxDecoration(
                  color: convPaneBackgroundColor.withValues(alpha: 0.9),
                  borderRadius: BorderRadius.circular(Spacings.m),
                ),
                child: IconButton(
                  icon: const Icon(Icons.close),
                  color: colorDMB,
                  hoverColor: const Color(0x00FFFFFF),
                  onPressed: () {
                    context.read<ConversationDetailsCubit>().resetDraft();
                    _inputController.clear();
                  },
                ),
              ),
            Container(
              width: 50,
              height: 50,
              margin: const EdgeInsets.only(left: Spacings.xs),
              decoration: BoxDecoration(
                color: convPaneBackgroundColor.withValues(alpha: 0.9),
                borderRadius: BorderRadius.circular(Spacings.m),
              ),
              child: IconButton(
                icon: Icon(_inputIsEmpty ? Icons.add : Icons.send),
                color: colorDMB,
                hoverColor: const Color(0x00FFFFFF),
                onPressed: () {
                  if (_inputIsEmpty) {
                    _uploadAttachment(context);
                  } else {
                    _submitMessage(context.read());
                  }
                },
              ),
            ),
          ],
        ),
      ),
    );
  }

  // Key events
  KeyEventResult _onKeyEvent(
    ConversationDetailsCubit conversationDetailCubit,
    FocusNode node,
    KeyEvent evt,
  ) {
    if (evt.logicalKey == LogicalKeyboardKey.enter &&
        evt is KeyDownEvent &&
        HardwareKeyboard.instance.logicalKeysPressed.length == 1) {
      _submitMessage(conversationDetailCubit);
      return KeyEventResult.handled;
    } else if (evt.logicalKey == LogicalKeyboardKey.arrowUp &&
        evt is KeyDownEvent &&
        HardwareKeyboard.instance.logicalKeysPressed.length == 1) {
      return _editMessage(conversationDetailCubit)
          ? KeyEventResult.handled
          : KeyEventResult.ignored;
    } else {
      return KeyEventResult.ignored;
    }
  }

  void _submitMessage(ConversationDetailsCubit conversationDetailsCubit) async {
    final messageText = _inputController.text.trim();
    if (messageText.isEmpty) {
      return;
    }

    // FIXME: Handle errors
    conversationDetailsCubit.sendMessage(messageText);

    setState(() {
      _inputController.clear();
      _focusNode.requestFocus();
    });
  }

  bool _editMessage(ConversationDetailsCubit cubit) {
    if (_inputController.text.trim().isNotEmpty) {
      return false;
    }
    if (cubit.state.conversation?.draft?.editingId != null) {
      return false;
    }
    cubit.editMessage();
    return true;
  }

  void _uploadAttachment(BuildContext context) async {
    final ImagePicker picker = ImagePicker();
    final XFile? file = await picker.pickImage(source: ImageSource.gallery);

    if (file == null) {
      return;
    }

    if (!context.mounted) {
      return;
    }

    final cubit = context.read<ConversationDetailsCubit>();
    try {
      await cubit.uploadAttachment(file.path);
    } catch (e) {
      _log.severe("Failed to upload attachment: $e");
      if (context.mounted) {
        final loc = AppLocalizations.of(context);
        showErrorBanner(
          ScaffoldMessenger.of(context),
          loc.composer_error_attachment,
        );
      }
    }
  }

  void _onTextChanged() {
    setState(() {
      _inputIsEmpty = _inputController.text.trim().isEmpty;
    });
    _storeDraftDebouncer.run(() {
      _conversationDetailsCubit.storeDraft(draftMessage: _inputController.text);
    });
  }
}

class _MessageInput extends StatelessWidget {
  const _MessageInput({
    required FocusNode focusNode,
    required TextEditingController controller,
    required this.conversationTitle,
    required this.isEditing,
  }) : _focusNode = focusNode,
       _controller = controller;

  final FocusNode _focusNode;
  final TextEditingController _controller;
  final String? conversationTitle;
  final bool isEditing;

  @override
  Widget build(BuildContext context) {
    final smallScreen = isSmallScreen(context);

    final loc = AppLocalizations.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (isEditing)
          Padding(
            padding: const EdgeInsets.only(
              top: Spacings.xs,
              left: Spacings.xxs,
              right: Spacings.xxs,
            ),
            child: Row(
              children: [
                const Icon(Icons.edit_outlined),
                const SizedBox(width: Spacings.xxs),
                Text(loc.composer_editMessage),
              ],
            ),
          ),
        TextField(
          focusNode: _focusNode,
          style: messageTextStyle(context, false),
          controller: _controller,
          minLines: 1,
          maxLines: 10,
          decoration: InputDecoration(
            hintText: loc.composer_inputHint(conversationTitle ?? ""),
            hintStyle: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: colorDMB),
          ).copyWith(filled: false),
          textInputAction:
              smallScreen ? TextInputAction.send : TextInputAction.newline,
          onEditingComplete: () => _focusNode.requestFocus(),
          keyboardType: TextInputType.multiline,
          textCapitalization: TextCapitalization.sentences,
        ),
      ],
    );
  }
}

enum Direction { right, left }
