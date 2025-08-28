// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:air/conversation_list/conversation_list_content.dart';
import 'package:air/conversation_list/conversation_list_cubit.dart';
import 'package:mocktail/mocktail.dart';
import 'package:air/core/api/markdown.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/app_localizations.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user.dart';

import '../../mocks.dart';
import '../../helpers.dart';

final userProfiles = [
  UiUserProfile(userId: 1.userId(), displayName: 'Alice'),
  UiUserProfile(userId: 2.userId(), displayName: 'Bob'),
  UiUserProfile(userId: 3.userId(), displayName: 'Eve'),
  UiUserProfile(userId: 4.userId(), displayName: 'Dave'),
  UiUserProfile(userId: 5.userId(), displayName: 'Frank'),
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
          edited: false,
          content: UiMimiContent(
            plainBody: 'Hey Alice, sorry for all the spam',
            topicId: Uint8List(0),
            content: simpleMessage('Hey Alice, sorry for all the spam'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiConversationDetails(
    id: 2.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: UiConversationType_Connection(userProfiles[2]),
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
          edited: true,
          content: UiMimiContent(
            plainBody: 'What is the recipe for the cake you made?',
            topicId: Uint8List(0),
            content: simpleMessage('What is the recipe for the cake you made?'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiConversationDetails(
    id: 3.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: const UiConversationType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(
      title: 'Science club',
      picture: null,
    ),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 3.conversationMessageId(),
      conversationId: 3.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 4.userId(),
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: "What is the distance to the nearest star?",
            topicId: Uint8List(0),
            content: simpleMessage("What is the distance to the nearest star?"),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiConversationDetails(
    id: 4.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: UiConversationType_Connection(userProfiles[3]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(title: 'Dave', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 2.conversationMessageId(),
      conversationId: 2.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          edited: true,
          content: UiMimiContent(
            plainBody: 'I have to tell you all about my weekend...',
            topicId: Uint8List(0),
            content: simpleMessage(
              'I have to tell you all about my weekend...',
            ),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiConversationDetails(
    id: 5.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: const UiConversationType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(
      title: 'Labubu fan club',
      picture: null,
    ),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 3.conversationMessageId(),
      conversationId: 3.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 4.userId(),
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: 'I found one for only \$800',
            topicId: Uint8List(0),
            content: simpleMessage('I found one for only \$800'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
    draft: UiMessageDraft(
      message: 'I found one for only \$800',
      editingId: null,
      updatedAt: DateTime.now(),
      source: UiMessageDraftSource.system,
    ),
  ),
  UiConversationDetails(
    id: 6.conversationId(),
    status: const UiConversationStatus.active(),
    conversationType: UiConversationType_Connection(userProfiles[4]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiConversationAttributes(title: 'Frank', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiConversationMessage(
      id: 2.conversationMessageId(),
      conversationId: 2.conversationId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          edited: true,
          content: UiMimiContent(
            plainBody: 'Going to the store. Need anything?',
            topicId: Uint8List(0),
            content: simpleMessage('Going to the store. Need anything?'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
];

MessageContent simpleMessage(String msg) {
  return MessageContent(
    elements: [
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
            theme: themeData(
              MediaQuery.platformBrightnessOf(context),
              CustomColorScheme.of(context),
            ),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
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

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_list_content.png'),
      );
    });
  });
}
