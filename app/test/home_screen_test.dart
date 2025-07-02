// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/conversation_list/conversation_list.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/home_screen.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/message_list/message_list.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:visibility_detector/visibility_detector.dart';

import 'conversation/conversation_screen_view_test.dart';
import 'conversation_list/conversation_list_content_test.dart';
import 'helpers.dart';
import 'message_list/message_list_test.dart';
import 'mocks.dart';

void main() {
  setUpAll(() {
    registerFallbackValue(0.conversationMessageId());
    registerFallbackValue(0.userId());
  });

  group('HomeScreen', () {
    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockConversationListCubit conversationListCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      conversationListCubit = MockConversationListCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();

      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(() => conversationDetailsCubit.state).thenReturn(
        ConversationDetailsState(conversation: conversation, members: members),
      );
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

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ConversationListCubit>.value(value: conversationListCubit),
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
            theme: themeData(context),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: const HomeScreenDesktopLayout(
              conversationList: ConversationListView(),
              conversation: ConversationScreenView(
                createMessageCubit: createMockMessageCubit,
              ),
            ),
          );
        },
      ),
    );

    testWidgets('desktop layout empty', (tester) async {
      final binding = TestWidgetsFlutterBinding.ensureInitialized();
      binding.platformDispatcher.views.first.physicalSize = const Size(
        3840,
        2160,
      );
      addTearDown(() {
        binding.platformDispatcher.views.first.resetPhysicalSize();
      });

      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(
        () => conversationListCubit.state,
      ).thenReturn(const ConversationListState(conversations: []));
      when(() => messageListCubit.state).thenReturn(MockMessageListState([]));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/home_screen_desktop_empty.png'),
      );
    });

    testWidgets('desktop layout no conversation', (tester) async {
      final binding = TestWidgetsFlutterBinding.ensureInitialized();
      binding.platformDispatcher.views.first.physicalSize = const Size(
        3840,
        2160,
      );
      addTearDown(() {
        binding.platformDispatcher.views.first.resetPhysicalSize();
      });

      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(
        () => conversationListCubit.state,
      ).thenReturn(ConversationListState(conversations: conversations));
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/home_screen_desktop_no_conversation.png'),
      );
    });

    testWidgets('desktop layout selected conversation', (tester) async {
      final binding = TestWidgetsFlutterBinding.ensureInitialized();
      binding.platformDispatcher.views.first.physicalSize = const Size(
        3840,
        2160,
      );
      addTearDown(() {
        binding.platformDispatcher.views.first.resetPhysicalSize();
      });

      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(
          home: HomeNavigationState(
            conversationOpen: true,
            conversationId: conversations[2].id,
          ),
        ),
      );
      when(
        () => conversationListCubit.state,
      ).thenReturn(ConversationListState(conversations: conversations));
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(messages));

      VisibilityDetectorController.instance.updateInterval = Duration.zero;

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/home_screen_desktop.png'),
      );
    });
  });
}
