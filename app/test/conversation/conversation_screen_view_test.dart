// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:air/conversation_details/conversation_details.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/message_list/message_list.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/user/user.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../conversation_list/conversation_list_content_test.dart';
import '../helpers.dart';
import '../message_list/message_list_test.dart';
import '../mocks.dart';

final chat = chats[2];

final members = [1.userId(), 2.userId(), 3.userId()];

final profiles = [
  UiUserProfile(userId: 1.userId(), displayName: 'Alice'),
  UiUserProfile(userId: 2.userId(), displayName: 'Bob'),
  UiUserProfile(userId: 3.userId(), displayName: 'Eve'),
];

void main() {
  setUpAll(() {
    registerFallbackValue(0.messageId());
    registerFallbackValue(0.userId());
  });

  group('ConversationScreenView', () {
    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();

      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(
        () => conversationDetailsCubit.state,
      ).thenReturn(ConversationDetailsState(chat: chat, members: members));
      when(
        () => conversationDetailsCubit.markAsRead(
          untilMessageId: any(named: "untilMessageId"),
          untilTimestamp: any(named: "untilTimestamp"),
        ),
      ).thenAnswer((_) => Future.value());
      when(
        () => conversationDetailsCubit.storeDraft(
          draftMessage: any(named: "draftMessage"),
        ),
      ).thenAnswer((_) async => Future.value());
      when(() => userSettingsCubit.state).thenReturn(const UserSettings());
    });

    Widget buildSubject({bool useDarkTheme = false}) => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ConversationDetailsCubit>.value(
          value: conversationDetailsCubit,
        ),
        BlocProvider<MessageListCubit>.value(value: messageListCubit),
        BlocProvider<UserSettingsCubit>.value(value: userSettingsCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: useDarkTheme ? darkTheme : lightTheme,
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: const Scaffold(
              body: ConversationScreenView(
                createMessageCubit: createMockMessageCubit,
              ),
            ),
          );
        },
      ),
    );

    testWidgets('renders correctly when empty', (tester) async {
      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(() => messageListCubit.state).thenReturn(MockMessageListState([]));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_screen_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chat.id)),
      );
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_screen.png'),
      );
    });

    testWidgets('renders correctly (dark mode)', (tester) async {
      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chat.id)),
      );
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject(useDarkTheme: true));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/conversation_screen_dark.png'),
      );
    });
  });
}
