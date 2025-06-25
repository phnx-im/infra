// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/message_list/message_list.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../conversation_list/conversation_list_content_test.dart';
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
        sender: 2.userId(),
        sent: true,
        content: UiMimiContent(
          plainBody: 'Hello Alice from Bob',
          topicId: Uint8List(0),
          content: simpleMessage('Hello Alice from Bob'),
          attachments: [],
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
        sender: 3.userId(),
        sent: true,
        content: UiMimiContent(
          plainBody:
              'Hello Alice. This is a long message that should not be truncated but properly split into multiple lines.',
          topicId: Uint8List(0),
          content: simpleMessage(
            'Hello Alice. This is a long message that should not be truncated but properly split into multiple lines.',
          ),
          attachments: [],
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
        sender: 1.userId(),
        sent: true,
        content: UiMimiContent(
          plainBody: 'Hello Bob and Eve',
          topicId: Uint8List(0),
          content: simpleMessage('Hello Bob and Eve'),
          attachments: [],
        ),
      ),
    ),
    position: UiFlightPosition.start,
  ),
  UiConversationMessage(
    id: 4.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:03:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: 1.userId(),
        sent: true,
        content: UiMimiContent(
          plainBody: 'How are you doing?',
          topicId: Uint8List(0),
          content: simpleMessage('How are you doing?'),
          attachments: [],
        ),
      ),
    ),
    position: UiFlightPosition.middle,
  ),
  UiConversationMessage(
    id: 5.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:03:00.000Z',
    message: UiMessage_Content(
      UiContentMessage(
        sender: 1.userId(),
        sent: true,
        content: UiMimiContent(
          plainBody: '''Nice to see you both here! 👋

This is a message with multiple lines. It should be properly displayed in the message bubble and split between multiple lines.''',
          topicId: Uint8List(0),
          content: simpleMessage(
            '''Nice to see you both here! 👋

This is a message with multiple lines. It should be properly displayed in the message bubble and split between multiple lines.''',
          ),
          attachments: [],
        ),
      ),
    ),
    position: UiFlightPosition.end,
  ),
];

final imageAttachment = UiAttachment(
  attachmentId: 2.attachmentId(),
  filename: "image.png",
  size: 10 * 1024 * 1024,
  contentType: 'image/png',
  blurhash: "LEHLk~WB2yk8pyo0adR*.7kCMdnj",
  description: "A woman eating a donut",
);

final attachmentMessages = [
  UiConversationMessage(
    id: 6.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:04:00.000Z',
    position: UiFlightPosition.start,
    message: UiMessage_Content(
      UiContentMessage(
        sender: 1.userId(),
        sent: true,
        content: UiMimiContent(
          topicId: Uint8List(0),
          plainBody: "A File Attachment",
          content: simpleMessage('A File Attachment'),
          attachments: [
            UiAttachment(
              attachmentId: 1.attachmentId(),
              filename: "file.zip",
              contentType: "application/zip",
              size: 1024,
              description: "Failing golden tests",
            ),
          ],
        ),
      ),
    ),
  ),
  UiConversationMessage(
    id: 7.conversationMessageId(),
    conversationId: conversationId,
    timestamp: '2023-01-01T00:04:01.000Z',
    position: UiFlightPosition.end,
    message: UiMessage_Content(
      UiContentMessage(
        sender: 1.userId(),
        sent: true,
        content: UiMimiContent(
          topicId: Uint8List(0),
          plainBody: "Look what I've got to eat",
          content: simpleMessage("Look what I've got to eat"),
          attachments: [imageAttachment],
        ),
      ),
    ),
  ),
];

MessageCubit createMockMessageCubit({
  required UserCubit userCubit,
  required MessageState initialState,
}) => MockMessageCubit(initialState: initialState);

void main() {
  setUpAll(() {
    registerFallbackValue(0.conversationMessageId());
    registerFallbackValue(0.userId());
  });

  group('MessageListView', () {
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockAttachmentsRepository attachmentsRepository;

    setUp(() async {
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();
      messageListCubit = MockMessageListCubit();
      attachmentsRepository = MockAttachmentsRepository();

      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(
        () => conversationDetailsCubit.markAsRead(
          untilMessageId: any(named: 'untilMessageId'),
          untilTimestamp: any(named: 'untilTimestamp'),
        ),
      ).thenAnswer((_) => Future.value());
    });

    Widget buildSubject() => RepositoryProvider<AttachmentsRepository>.value(
      value: attachmentsRepository,
      child: MultiBlocProvider(
        providers: [
          BlocProvider<UserCubit>.value(value: userCubit),
          BlocProvider<UsersCubit>.value(value: contactsCubit),
          BlocProvider<ConversationDetailsCubit>.value(
            value: conversationDetailsCubit,
          ),
          BlocProvider<MessageListCubit>.value(value: messageListCubit),
        ],
        child: Builder(
          builder: (context) {
            return MaterialApp(
              debugShowCheckedModeBanner: false,
              theme: themeData(context),
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              home: const Scaffold(
                body: MessageListView(
                  createMessageCubit: createMockMessageCubit,
                ),
              ),
            );
          },
        ),
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
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/message_list.png'),
      );
    });

    testWidgets('renders correctly with attachments', (tester) async {
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages + attachmentMessages));
      when(
        () => attachmentsRepository.loadAttachment(
          attachmentId: imageAttachment.attachmentId,
        ),
      ).thenAnswer((_) async => Future.any([]));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/message_list_attachments.png'),
      );
    });
  });
}
