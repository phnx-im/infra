// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/app_localizations.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'conversation_list_cubit.dart';

class ConversationListContent extends StatelessWidget {
  const ConversationListContent({super.key});

  @override
  Widget build(BuildContext context) {
    final conversations = context.select(
      (ConversationListCubit cubit) => cubit.state.conversations,
    );

    if (conversations.isEmpty) {
      return const _NoConversations();
    }

    return ListView.builder(
      padding: const EdgeInsets.all(0),
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
    final currentConversationId = context.select(
      (NavigationCubit cubit) => cubit.state.openConversationId,
    );
    final isSelected = currentConversationId == conversation.id;
    return ListTile(
      horizontalTitleGap: 0,
      contentPadding: const EdgeInsets.symmetric(
        horizontal: Spacings.xxs,
        vertical: Spacings.xxxs,
      ),
      minVerticalPadding: 0,
      title: Container(
        alignment: AlignmentDirectional.centerStart,
        height: 70,
        width: 300,
        padding: const EdgeInsets.symmetric(
          horizontal: Spacings.xs,
          vertical: Spacings.xxs,
        ),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(Spacings.s),
          color: isSelected ? convPaneFocusColor : null,
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.center,
          spacing: Spacings.s,
          children: [
            UserAvatar(
              size: 48,
              image: conversation.picture,
              displayName: conversation.title,
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.center,
                mainAxisAlignment: MainAxisAlignment.start,
                children: [
                  _ListTileTop(conversation: conversation),
                  Expanded(child: _ListTileBottom(conversation: conversation)),
                ],
              ),
            ),
          ],
        ),
      ),
      selected: isSelected,
      focusColor: convListItemSelectedColor,
      onTap:
          () =>
              context.read<NavigationCubit>().openConversation(conversation.id),
    );
  }
}

class _ListTileTop extends StatelessWidget {
  const _ListTileTop({required this.conversation});

  final UiConversationDetails conversation;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      spacing: Spacings.xxs,
      children: [
        Expanded(child: _ConversationTitle(title: conversation.title)),
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
    final ownClientId = context.select((UserCubit cubit) => cubit.state.userId);

    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: Spacings.s,
      children: [
        Expanded(
          child: Align(
            alignment: Alignment.topLeft,
            child: _LastMessage(
              conversation: conversation,
              ownClientId: ownClientId,
            ),
          ),
        ),
        Align(
          alignment: Alignment.center,
          child: _UnreadBadge(
            conversationId: conversation.id,
            count: conversation.unreadMessages,
          ),
        ),
      ],
    );
  }
}

class _UnreadBadge extends StatelessWidget {
  const _UnreadBadge({required this.conversationId, required this.count});

  final ConversationId conversationId;
  final int count;

  @override
  Widget build(BuildContext context) {
    final currentConversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    if (count < 1 || conversationId == currentConversationId) {
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
          letterSpacing: 0,
        ).merge(VariableFontWeight.semiBold),
      ),
    );
  }
}

class _LastMessage extends StatelessWidget {
  const _LastMessage({required this.conversation, required this.ownClientId});

  final UiConversationDetails conversation;
  final UiUserId ownClientId;

  @override
  Widget build(BuildContext context) {
    final currentConversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    final lastMessage = conversation.lastMessage;

    final readStyle = TextStyle(
      color: colorDMB,
      fontSize: isSmallScreen(context) ? 14 : 13,
      height: 1.2,
    ).merge(VariableFontWeight.normal);
    final unreadStyle = readStyle.merge(VariableFontWeight.medium);

    final contentStyle =
        conversation.id != currentConversationId &&
                conversation.unreadMessages > 0
            ? unreadStyle
            : readStyle;

    final senderStyle = readStyle.merge(VariableFontWeight.semiBold);

    final (sender, displayedLastMessage) = switch (lastMessage?.message) {
      UiMessage_Content(field0: final content) => (
        content.sender == ownClientId ? 'You: ' : null,
        content.content.plainBody,
      ),
      UiMessage_Display() => (null, null),
      null => (null, null),
    };

    return Text.rich(
      maxLines: 2,
      softWrap: true,
      overflow: TextOverflow.ellipsis,
      TextSpan(
        text: sender,
        style: senderStyle,
        children: [TextSpan(text: displayedLastMessage, style: contentStyle)],
      ),
    );
  }
}

class _LastUpdated extends StatefulWidget {
  const _LastUpdated({required this.conversation});

  final UiConversationDetails conversation;

  @override
  State<_LastUpdated> createState() => _LastUpdatedState();
}

class _LastUpdatedState extends State<_LastUpdated> {
  String _displayTimestamp = '';
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _displayTimestamp = formatTimestamp(widget.conversation.lastUsed);
    _timer = Timer.periodic(const Duration(seconds: 5), (timer) {
      final newDisplayTimestamp = formatTimestamp(widget.conversation.lastUsed);
      if (newDisplayTimestamp != _displayTimestamp) {
        setState(() {
          _displayTimestamp = newDisplayTimestamp;
        });
      }
    });
  }

  @override
  void didUpdateWidget(covariant _LastUpdated oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.conversation.lastUsed != widget.conversation.lastUsed) {
      setState(() {
        _displayTimestamp = formatTimestamp(widget.conversation.lastUsed);
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return Baseline(
      baseline: Spacings.xs,
      baselineType: TextBaseline.alphabetic,
      child: Text(
        _localizedTimestamp(_displayTimestamp, loc),
        style: const TextStyle(
          color: colorDMB,
          fontSize: 12,
          letterSpacing: -0.2,
        ).merge(VariableFontWeight.medium),
      ),
    );
  }
}

class _ConversationTitle extends StatelessWidget {
  const _ConversationTitle({required this.title});

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
        ).merge(VariableFontWeight.semiBold),
      ),
    );
  }
}

String _localizedTimestamp(String original, AppLocalizations loc) =>
    switch (original) {
      'Now' => loc.timestamp_now,
      'Yesterday' => loc.timestamp_yesterday,
      _ => original,
    };

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
