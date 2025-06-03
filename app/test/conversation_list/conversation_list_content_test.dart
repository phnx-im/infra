// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:prototype/conversation_list/conversation_list_content.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/core/api/markdown.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';

import '../mocks.dart';
import '../helpers.dart';

final userProfiles = [
  UiUserProfile(userId: 1.userId(), displayName: 'alice'),
  UiUserProfile(userId: 2.userId(), displayName: 'bob'),
  UiUserProfile(userId: 3.userId(), displayName: 'eve'),
];

final conversations = [
  UiConversationDetails(
    id: 1.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: UiConversationType_Connection(userProfiles[1]),
    unreadMessages: 10,
    messagesCount: 10,
    attributes: const UiConversationAttributes(title: 'Bob', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 1.conversationMessageId(),
      conversationId: 1.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 2.userId(),
          sent: true,
          content: UiMimiContent(
            plainBody: 'Hello Alice',
            topicId: Uint8List(0),
            content: simpleMessage('Hello Alice'),
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
  UiConversationDetails(
    id: 2.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: UiConversationType_UnconfirmedConnection(userProfiles[2]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(title: 'Eve', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 2.conversationMessageId(),
      conversationId: 2.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          content: UiMimiContent(
            plainBody:
                'Hello Alice. This is a long message that should not be truncated but properly split into multiple lines.',
            topicId: Uint8List(0),
            content: simpleMessage(
              'Hello Alice. This is a long message that should not be truncated but properly split into multiple lines.',
            ),
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
  UiConversationDetails(
    id: 3.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: const UiConversationType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(title: 'Group', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 3.conversationMessageId(),
      conversationId: 3.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 4.userId(),
          sent: true,
          content: UiMimiContent(
            plainBody: 'Hello All',
            topicId: Uint8List(0),
            content: simpleMessage('Hello All'),
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
];

MessageContent simpleMessage(String msg) {
  return MessageContent(
    content: [
      RangedBlockElement(
        start: 0,
        end: msg.length,
        element: BlockElement_Paragraph([
          RangedInlineElement(
            start: 0,
            end: msg.length,
            element: InlineElement_Text(msg),
          ),
        ]),
      ),
    ],
  );
}

void main() {
  group('ConversationListContent', () {
    late MockNavigationCubit navigationCubit;
    late MockConversationListCubit conversationListCubit;
    late MockUserCubit userCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      conversationListCubit = MockConversationListCubit();

      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<ConversationListCubit>.value(value: conversationListCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: themeData(context),
            home: const Scaffold(body: ConversationListContent()),
          );
        },
      ),
    );

    testWidgets('renders correctly when there are no conversations', (
      tester,
    ) async {
      when(
        () => conversationListCubit.state,
      ).thenReturn(const ConversationListState(conversations: []));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_list_content_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(
          home: HomeNavigationState(
            conversationOpen: true,
            conversationId: conversations[1].id,
          ),
        ),
      );
      when(() => conversationListCubit.state).thenReturn(
        ConversationListState(
          conversations: List.generate(
            20,
            (index) => conversations[index % conversations.length],
          ),
        ),
      );

      await tester.pumpWidget(buildSubject());

      // Increase threshold because rendering frosted glass varies significantly across different platforms.
      await withThreshold(0.031, () async {
        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/conversation_list_content.png'),
        );
      });
    });
  });
}
