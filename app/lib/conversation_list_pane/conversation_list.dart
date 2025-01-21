// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user_cubit.dart';
import 'package:provider/provider.dart';

import 'conversation_list_cubit.dart';

final _log = Logger("ConversationList");

class ConversationList extends StatelessWidget {
  const ConversationList({super.key});

  @override
  Widget build(BuildContext context) {
    final conversations = context
        .select((ConversationListCubit cubit) => cubit.state.conversations);

    if (conversations.isEmpty) {
      return const _NoConversations();
    }

    return ListView.builder(
      itemCount: conversations.length,
      physics: const BouncingScrollPhysics().applyTo(
        const AlwaysScrollableScrollPhysics(),
      ),
      itemBuilder: (BuildContext context, int index) {
        return _ListTile(conversation: conversations[index]);
      },
    );
  }
}

class _NoConversations extends StatelessWidget {
  const _NoConversations();

  @override
  Widget build(BuildContext context) {
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

class _ListTile extends StatelessWidget {
  const _ListTile({required this.conversation});

  final UiConversationDetails conversation;

  @override
  Widget build(BuildContext context) {
    final currentConversationId =
        context.select((NavigationCubit cubit) => cubit.state.conversationId);
    return ListTile(
      horizontalTitleGap: 0,
      contentPadding: const EdgeInsets.symmetric(
        horizontal: Spacings.xxs,
        vertical: Spacings.xxxs,
      ),
      minVerticalPadding: 0,
      title: Container(
        alignment: AlignmentDirectional.topStart,
        height: 74,
        width: 300,
        padding: const EdgeInsets.all(10),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(10),
          color: _selectionColor(
            context,
            conversation.id,
            currentConversationId,
          ),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            UserAvatar(
              size: 48,
              cacheTag: conversation.avatarCacheTag,
              image: conversation.attributes.conversationPictureOption,
              username: conversation.username,
            ),
            const SizedBox(width: Spacings.s),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.center,
                mainAxisAlignment: MainAxisAlignment.start,
                children: [
                  _ListTileTop(conversation: conversation),
                  const SizedBox(height: 2),
                  Expanded(child: _ListTileBottom(conversation: conversation)),
                ],
              ),
            ),
          ],
        ),
      ),
      selected: _isConversationSelected(
        conversation.id,
        currentConversationId,
        context,
      ),
      focusColor: convListItemSelectedColor,
      onTap: () => _onSelectConversation(context, conversation.id),
    );
  }

  void _onSelectConversation(
    BuildContext context,
    ConversationId conversationId,
  ) {
    _log.info("Tapped on conversation $conversationId");
    context.read<CoreClient>().selectConversation(conversationId);
    context.read<NavigationCubit>().openConversation(conversationId);
  }

  Color? _selectionColor(
    BuildContext context,
    ConversationId conversationId,
    ConversationId? currentConversationId,
  ) =>
      isLargeScreen(context) && currentConversationId == conversationId
          ? convPaneFocusColor
          : null;
}

class _ListTileTop extends StatelessWidget {
  const _ListTileTop({required this.conversation});

  final UiConversationDetails conversation;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(child: _ConversationTitle(title: conversation.title)),
        const SizedBox(width: 8),
        _LastUpdated(conversation: conversation),
      ],
    );
  }
}

class _ListTileBottom extends StatelessWidget {
  const _ListTileBottom({required this.conversation});

  final UiConversationDetails conversation;

  @override
  Widget build(BuildContext context) {
    final userName = context.select((UserCubit cubit) => cubit.state.userName);

    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(
          child: Align(
            alignment: Alignment.topLeft,
            child: _LastMessage(conversation: conversation, userName: userName),
          ),
        ),
        const SizedBox(width: 16),
        Align(
          alignment: Alignment.center,
          child: _UnreadBadge(count: conversation.unreadMessages),
        ),
      ],
    );
  }
}

class _UnreadBadge extends StatelessWidget {
  const _UnreadBadge({
    required this.count,
  });

  final int count;

  @override
  Widget build(BuildContext context) {
    if (count < 1) {
      return const SizedBox();
    }
    final badgeText = count <= 100 ? "$count" : "100+";
    const double badgeSize = 20;
    return Container(
      alignment: AlignmentDirectional.center,
      constraints: const BoxConstraints(minWidth: badgeSize),
      padding: const EdgeInsets.fromLTRB(7, 3, 7, 4),
      height: badgeSize,
      decoration: BoxDecoration(
        color: colorDMB,
        borderRadius: BorderRadius.circular(badgeSize / 2),
      ),
      child: Text(
        badgeText,
        style: const TextStyle(
            color: Colors.white,
            fontSize: 10,
            fontVariations: variationSemiBold,
            letterSpacing: 0),
      ),
    );
  }
}

class _LastMessage extends StatelessWidget {
  const _LastMessage({
    required this.conversation,
    required this.userName,
  });

  final UiConversationDetails conversation;
  final String userName;

  @override
  Widget build(BuildContext context) {
    final lastMessage = conversation.lastMessage;
    final style = TextStyle(
      color: colorDMB,
      fontSize: isSmallScreen(context) ? 14 : 13,
      fontVariations: variationRegular,
      letterSpacing: -0.2,
      height: 1.2,
    );

    final contentStyle = conversation.unreadMessages > 0
        ? style.copyWith(fontVariations: variationMedium)
        : style;

    final senderStyle = style.copyWith(fontVariations: variationSemiBold);

    final (sender, displayedLastMessage) = switch (lastMessage?.message) {
      UiMessage_Content(field0: final content) => (
          content.sender == userName ? 'You: ' : null,
          content.content.body
        ),
      UiMessage_Display() => (null, null),
      UiMessage_Unsent(field0: final unsent) => (
          null,
          '⚠️ Unsent message: ${unsent.body}'
        ),
      null => (null, null),
    };

    return Text.rich(
      maxLines: 2,
      softWrap: true,
      overflow: TextOverflow.ellipsis,
      TextSpan(
        text: sender,
        style: senderStyle,
        children: [
          TextSpan(
            text: displayedLastMessage,
            style: contentStyle,
          ),
        ],
      ),
    );
  }
}

class _LastUpdated extends StatelessWidget {
  const _LastUpdated({required this.conversation});

  final UiConversationDetails conversation;

  @override
  Widget build(BuildContext context) {
    return Baseline(
      baseline: Spacings.xs,
      baselineType: TextBaseline.alphabetic,
      child: Text(
        formatTimestamp(conversation.lastUsed),
        style: const TextStyle(
          color: colorDMB,
          fontSize: 11,
          fontVariations: variationRegular,
          letterSpacing: -0.2,
        ),
      ),
    );
  }
}

class _ConversationTitle extends StatelessWidget {
  const _ConversationTitle({
    required this.title,
  });

  final String title;

  @override
  Widget build(BuildContext context) {
    return Baseline(
      baseline: Spacings.s,
      baselineType: TextBaseline.alphabetic,
      child: Text(
        title,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(
          color: convListItemTextColor,
          fontSize: 14,
          fontVariations: variationSemiBold,
          letterSpacing: -0.2,
        ),
      ),
    );
  }
}

bool _isConversationSelected(
  ConversationId conversationId,
  ConversationId? currentConversationId,
  BuildContext context,
) {
  return isLargeScreen(context)
      ? currentConversationId == conversationId
      : false;
}

String formatTimestamp(String t, {DateTime? now}) {
  DateTime timestamp;
  try {
    timestamp = DateTime.parse(t);
  } catch (e) {
    return '';
  }

  now ??= DateTime.now();

  now = now.toLocal();

  final difference = now.difference(timestamp);
  final yesterday = DateTime(now.year, now.month, now.day - 1);

  if (difference.inSeconds < 60) {
    return 'Now';
  } else if (difference.inMinutes < 60) {
    return '${difference.inMinutes}m';
  } else if (now.year == timestamp.year &&
      now.month == timestamp.month &&
      now.day == timestamp.day) {
    return DateFormat('HH:mm').format(timestamp);
  } else if (now.year == timestamp.year &&
      timestamp.year == yesterday.year &&
      timestamp.month == yesterday.month &&
      timestamp.day == yesterday.day) {
    return 'Yesterday';
  } else if (difference.inDays < 7) {
    return DateFormat('E').format(timestamp);
  } else if (now.year == timestamp.year) {
    return DateFormat('dd.MM').format(timestamp);
  } else {
    return DateFormat('dd.MM.yy').format(timestamp);
  }
}
