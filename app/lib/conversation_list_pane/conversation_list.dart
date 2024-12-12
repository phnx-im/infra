// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/theme/theme.dart';
import 'package:convert/convert.dart';
import 'package:collection/collection.dart';
import 'package:prototype/user_cubit.dart';
import 'package:provider/provider.dart';

class ConversationList extends StatefulWidget {
  const ConversationList({super.key});

  @override
  State<ConversationList> createState() => _ConversationListState();
}

class _ConversationListState extends State<ConversationList> {
  _ConversationListState();

  late List<UiConversationDetails> _conversations;
  UiConversationDetails? _currentConversation;
  late final StreamSubscription<ConversationIdBytes>
      _conversationListUpdateListener;
  late final StreamSubscription<UiConversationDetails>
      _conversationSwitchListener;
  final ScrollController _scrollController = ScrollController();

  static const double _topBaseline = 12;

  @override
  void initState() {
    super.initState();

    final coreClient = context.coreClient;
    _conversations = coreClient.conversationsList;
    _currentConversation = coreClient.currentConversation;
    _conversationListUpdateListener = coreClient.onConversationListUpdate
        .listen(conversationListUpdateListener);

    final navigationCubit = context.read<NavigationCubit>();
    _conversationSwitchListener = coreClient.onConversationSwitch.listen(
      (conversationDetails) =>
          conversationSwitchListener(navigationCubit, conversationDetails),
    );

    updateConversationList(coreClient);
  }

  @override
  void dispose() {
    _conversationListUpdateListener.cancel();
    _conversationSwitchListener.cancel();
    super.dispose();
  }

  void conversationSwitchListener(
    NavigationCubit navigationCubit,
    UiConversationDetails cc,
  ) {
    if (_currentConversation != null) {
      if (_currentConversation!.id != cc.id) {
        setState(() {
          _currentConversation = cc;
        });
      }
    } else {
      _currentConversation = cc;
    }
    navigationCubit.openConversation(cc.id);
  }

  void selectConversation(
    CoreClient coreClient,
    ConversationIdBytes conversationId,
  ) {
    print("Tapped on conversation ${hex.encode(conversationId.bytes)}");
    coreClient.selectConversation(conversationId);
    context.read<NavigationCubit>().openConversation(conversationId);
  }

  void conversationListUpdateListener(ConversationIdBytes uuid) async {
    updateConversationList(context.coreClient);
  }

  void updateConversationList(CoreClient coreClient) async {
    await coreClient.conversations().then((conversations) {
      setState(() {
        if (_currentConversation == null && conversations.isNotEmpty) {
          coreClient.selectConversation(conversations[0].id);
        }
        _conversations = conversations;
      });
    });
  }

  Color? _selectionColor(int index) {
    if (isLargeScreen(context) &&
        _currentConversation != null &&
        _currentConversation!.id.bytes.equals(_conversations[index].id.bytes)) {
      return convPaneFocusColor;
    }
    return null;
  }

  Widget _userAvatar(int index) {
    return UserAvatar(
      size: 48,
      cacheTag: "conv:${hex.encode(_conversations[index].id.bytes)}",
      image: _conversations[index].attributes.conversationPictureOption,
      username: _conversations[index].conversationType.when(
          unconfirmedConnection: (e) => e,
          connection: (e) => e,
          group: () => _conversations[index].attributes.title),
    );
  }

  Widget _convTitle(int index) {
    return Baseline(
      baseline: Spacings.s,
      baselineType: TextBaseline.alphabetic,
      child: Text(
        _conversations[index].conversationType.when(
            unconfirmedConnection: (e) => '⏳ $e',
            connection: (e) => e,
            group: () => _conversations[index].attributes.title),
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

  Widget _lastMessage(String userName, int index) {
    var sender = '';
    var displayedLastMessage = '';
    final lastMessage = _conversations[index].lastMessage;
    final style = TextStyle(
      color: colorDMB,
      fontSize: isSmallScreen(context) ? 14 : 13,
      fontVariations: variationRegular,
      letterSpacing: -0.2,
      height: 1.2,
    );

    final contentStyle = _conversations[index].unreadMessages > 0
        ? style.copyWith(fontVariations: variationMedium)
        : style;

    final senderStyle = style.copyWith(fontVariations: variationSemiBold);

    if (lastMessage != null) {
      lastMessage.message.when(
          contentFlight: (c) {
            final lastContentMessage = c.last;
            if (lastContentMessage.sender == userName) {
              sender = 'You: ';
            }
            displayedLastMessage = lastContentMessage.content.body;
          },
          display: (d) => '',
          unsent: (u) => '⚠️ Unsent message: ${u.body}');
    }

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

  Widget _unreadBadge(int index) {
    final count = _conversations[index].unreadMessages;
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

  Widget _lastUpdated(int index) {
    return Baseline(
      baseline: _topBaseline,
      baselineType: TextBaseline.alphabetic,
      child: Text(
        formatTimestamp(_conversations[index].lastUsed),
        style: const TextStyle(
          color: colorDMB,
          fontSize: 11,
          fontVariations: variationRegular,
          letterSpacing: -0.2,
        ),
      ),
    );
  }

  Widget _topPart(int index) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(child: _convTitle(index)),
        const SizedBox(width: 8),
        _lastUpdated(index),
      ],
    );
  }

  Widget _bottomPart(String userName, int index) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(
          child: Align(
            alignment: Alignment.topLeft,
            child: _lastMessage(userName, index),
          ),
        ),
        const SizedBox(width: 16),
        Align(
          alignment: Alignment.center,
          child: _unreadBadge(index),
        ),
      ],
    );
  }

  Widget _listTile(String userName, int index) {
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
          color: _selectionColor(index),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _userAvatar(index),
            const SizedBox(width: Spacings.s),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.center,
                mainAxisAlignment: MainAxisAlignment.start,
                children: [
                  _topPart(index),
                  const SizedBox(height: 2),
                  Expanded(child: _bottomPart(userName, index)),
                ],
              ),
            ),
          ],
        ),
      ),
      selected: isConversationSelected(
          _currentConversation, _conversations[index], context),
      focusColor: convListItemSelectedColor,
      onTap: () => selectConversation(
        context.coreClient,
        _conversations[index].id,
      ),
    );
  }

  Widget _placeholder() {
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

  @override
  Widget build(BuildContext context) {
    final userName = context.select((UserCubit cubit) => cubit.state.userName);

    if (_conversations.isNotEmpty) {
      return ListView.builder(
        itemCount: _conversations.length,
        physics: const BouncingScrollPhysics().applyTo(
          const AlwaysScrollableScrollPhysics(),
        ),
        controller: _scrollController,
        itemBuilder: (BuildContext context, int index) {
          return _listTile(userName, index);
        },
      );
    } else {
      return _placeholder();
    }
  }
}

bool isConversationSelected(UiConversationDetails? currentConversation,
    UiConversationDetails conversation, BuildContext context) {
  if (isLargeScreen(context) && currentConversation != null) {
    return currentConversation.id.bytes.equals(conversation.id.bytes);
  }
  return false;
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
