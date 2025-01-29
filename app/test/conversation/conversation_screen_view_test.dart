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
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../conversation_list/conversation_list_content_test.dart';
import '../helpers.dart';
import '../message_list/message_list_test.dart';
import '../mocks.dart';

final conversation = conversations[2];

final members = [
  "alice@localhost",
  "bob@localhost",
  "eve@localhost",
];

void main() {
  setUpAll(() {
    registerFallbackValue(0.conversationMessageId());
  });

  group('ConversationScreenView', () {
    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockMessageListCubit messageListCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();
      messageListCubit = MockMessageListCubit();

      when(() => userCubit.state)
          .thenReturn(MockUiUser(userName: "alice@localhost"));
      when(() => userCubit.userProfile(any()))
          .thenAnswer((_) => Future.value(null));
      when(() => conversationDetailsCubit.state).thenReturn(
        ConversationDetailsState(
          conversation: conversation,
          members: members,
        ),
      );
      when(() => conversationDetailsCubit.markAsRead(
            untilMessageId: any(named: "untilMessageId"),
            untilTimestamp: any(named: "untilTimestamp"),
          )).thenAnswer((_) => Future.value());
    });

    Widget buildSubject() => MultiBlocProvider(
          providers: [
            BlocProvider<NavigationCubit>.value(
              value: navigationCubit,
            ),
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
                  body: ConversationScreenView(
                    createMessageCubit: createMockMessageCubit,
                  ),
                ),
              );
            },
          ),
        );

    testWidgets('renders correctly when empty', (tester) async {
      when(() => navigationCubit.state).thenReturn(NavigationState.home());
      when(() => messageListCubit.state).thenReturn(MockMessageListState([]));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_screen_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => navigationCubit.state)
          .thenReturn(NavigationState.home(conversationId: conversation.id));
      when(() => messageListCubit.state)
          .thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_screen.png'),
      );
    });
  });
}
