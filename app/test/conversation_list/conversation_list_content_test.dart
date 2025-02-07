// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:prototype/conversation_list/conversation_list_content.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';

import '../mocks.dart';
import '../helpers.dart';

final conversations = [
  UiConversationDetails(
    id: 1.conversationId(),
    status: UiConversationStatus.active(),
    conversationType: UiConversationType_Connection("bob@localhost"),
    unreadMessages: 10,
    messagesCount: 10,
    attributes: UiConversationAttributes(
      title: "Bob",
      picture: null,
    ),
    lastUsed: "2023-01-01T00:00:00.000Z",
    lastMessage: UiConversationMessage(
      id: 1.conversationMessageId(),
      conversationId: 1.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: "bob@localhost",
          sent: true,
          content: UiMimiContent(
            id: 1.messageId(),
            timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
            lastSeen: [],
            body: 'Hello Alice',
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
  UiConversationDetails(
    id: 2.conversationId(),
    status: UiConversationStatus.active(),
    conversationType: UiConversationType_UnconfirmedConnection("eve@localhost"),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: UiConversationAttributes(
      title: "Eve",
      picture: null,
    ),
    lastUsed: "2023-01-01T00:00:00.000Z",
    lastMessage: UiConversationMessage(
      id: 2.conversationMessageId(),
      conversationId: 2.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: "eve@localhost",
          sent: true,
          content: UiMimiContent(
            id: 2.messageId(),
            timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
            lastSeen: [],
            body:
                'Hello Alice. This is a long message that should not be truncated but properly split into multiple lines.',
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
  UiConversationDetails(
    id: 3.conversationId(),
    status: UiConversationStatus.active(),
    conversationType: UiConversationType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: UiConversationAttributes(
      title: "Group",
      picture: null,
    ),
    lastUsed: "2023-01-01T00:00:00.000Z",
    lastMessage: UiConversationMessage(
      id: 3.conversationMessageId(),
      conversationId: 3.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: "somebody@localhost",
          sent: true,
          content: UiMimiContent(
            id: 3.messageId(),
            timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
            lastSeen: [],
            body: 'Hello All',
          ),
        ),
      ),
      position: UiFlightPosition.single,
    ),
  ),
];

void main() {
  group('ConversationListContent', () {
    late MockNavigationCubit navigationCubit;
    late MockConversationListCubit conversationListCubit;
    late MockUserCubit userCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      conversationListCubit = MockConversationListCubit();

      when(() => navigationCubit.state).thenReturn(NavigationState.home());
      when(() => userCubit.state)
          .thenReturn(MockUiUser(userName: "alice@localhost"));
    });

    Widget buildSubject() => MultiBlocProvider(
          providers: [
            BlocProvider<NavigationCubit>.value(
              value: navigationCubit,
            ),
            BlocProvider<UserCubit>.value(
              value: userCubit,
            ),
            BlocProvider<ConversationListCubit>.value(
              value: conversationListCubit,
            ),
          ],
          child: Builder(
            builder: (context) {
              return MaterialApp(
                debugShowCheckedModeBanner: false,
                theme: themeData(context),
                home: Scaffold(body: ConversationListContent()),
              );
            },
          ),
        );

    testWidgets('renders correctly when there are no conversations',
        (tester) async {
      when(() => conversationListCubit.state)
          .thenReturn(ConversationListState(conversations: []));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_list_content_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => navigationCubit.state).thenReturn(
          NavigationState.home(conversationId: conversations[1].id));
      when(() => conversationListCubit.state).thenReturn(
        ConversationListState(
          conversations: List.generate(
              20, (index) => conversations[index % conversations.length]),
        ),
      );

      await tester.pumpWidget(buildSubject());

      // Increase threshold because rendering frosted glass varies significantly across different platforms.
      await withThreshold(0.03, () async {
        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/conversation_list_content.png'),
        );
      });
    });
  });
}
