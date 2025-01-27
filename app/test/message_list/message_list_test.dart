// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/message_list/message_list.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../helpers.dart';
import '../mocks.dart';

final conversationId = 1.conversationId();

final messages = [
  UiConversationMessage(
    id: 1.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:00:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: "bob@localhost",
        sent: true,
        content: UiMimiContent(
          id: 1.messageId(),
          timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
          lastSeen: [],
          body: 'Hello Alice from Bob',
        ),
      ),
    ),
    position: UiFlightPosition.single,
  ),
  UiConversationMessage(
    id: 2.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:01:00.000Z',
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
  UiConversationMessage(
    id: 3.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:02:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: "alice@localhost",
        sent: true,
        content: UiMimiContent(
          id: 3.messageId(),
          timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
          lastSeen: [],
          body: 'Hello Bob and Eve',
        ),
      ),
    ),
    position: UiFlightPosition.start,
  ),
  UiConversationMessage(
    id: 5.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:03:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: "alice@localhost",
        sent: true,
        content: UiMimiContent(
          id: 5.messageId(),
          timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
          lastSeen: [],
          body: 'How are you doing?',
        ),
      ),
    ),
    position: UiFlightPosition.middle,
  ),
  UiConversationMessage(
    id: 4.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:03:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: "alice@localhost",
        sent: true,
        content: UiMimiContent(
          id: 4.messageId(),
          timestamp: DateTime.parse("2023-01-01T00:00:00.000Z"),
          lastSeen: [],
          body: """Nice to see you both here! 👋

This is a message with multiple lines. It should be properly displayed in the message buble and split between multiple lines.""",
        ),
      ),
    ),
    position: UiFlightPosition.end,
  ),
];

MessageCubit createMockMessageCubit({
  required UserCubit userCubit,
  required MessageState initialState,
}) =>
    MockMessageCubit(initialState: initialState);

void main() {
  setUpAll(() {
    registerFallbackValue(0.conversationMessageId());
  });

  group('MessageListView', () {
    late MockUserCubit userCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockMessageListCubit messageListCubit;

    setUp(() async {
      userCubit = MockUserCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();
      messageListCubit = MockMessageListCubit();

      when(() => userCubit.state)
          .thenReturn(MockUiUser(userName: "alice@localhost"));
      when(() => userCubit.userProfile(any()))
          .thenAnswer((_) => Future.value(null));
      when(() => conversationDetailsCubit.markAsRead(
            untilMessageId: any(named: "untilMessageId"),
            untilTimestamp: any(named: "untilTimestamp"),
          )).thenAnswer((_) => Future.value());
    });

    Widget buildSubject() => MultiBlocProvider(
          providers: [
            BlocProvider<UserCubit>.value(
              value: userCubit,
            ),
            BlocProvider<ConversationDetailsCubit>.value(
              value: conversationDetailsCubit,
            ),
            BlocProvider<MessageListCubit>.value(
              value: messageListCubit,
            ),
          ],
          child: Builder(
            builder: (context) {
              return MaterialApp(
                debugShowCheckedModeBanner: false,
                theme: themeData(context),
                home: Scaffold(
                  body: MessageListView(
                    createMessageCubit: createMockMessageCubit,
                  ),
                ),
              );
            },
          ),
        );

    testWidgets('renders correctly when empty', (tester) async {
      when(() => messageListCubit.state).thenReturn(MockMessageListState([]));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/message_list_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => messageListCubit.state)
          .thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/message_list.png'),
      );
    });
  });
}
