// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:air/conversation_list/conversation_list.dart';
import 'package:mocktail/mocktail.dart';
import 'package:air/conversation_list/conversation_list_cubit.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/user/user.dart';

import '../helpers.dart';
import '../mocks.dart';
import 'conversation_list_content_test.dart';

void main() {
  group('ConversationList', () {
    late MockNavigationCubit navigationCubit;
    late MockConversationListCubit conversationListCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockConversationDetailsCubit conversationDetailsCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      conversationListCubit = MockConversationListCubit();
      contactsCubit = MockUsersCubit();
      conversationDetailsCubit = MockConversationDetailsCubit();

      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(() => contactsCubit.state).thenReturn(
        MockUsersState(
          profiles: [UiUserProfile(userId: 1.userId(), displayName: "alice")],
        ),
      );
      when(() => conversationDetailsCubit.state).thenReturn(
        ConversationDetailsState(
          conversation: conversations[1],
          members: [1.userId()],
        ),
      );
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
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
            home: const Scaffold(body: ConversationListView()),
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
        matchesGoldenFile('goldens/conversation_list_empty.png'),
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
        matchesGoldenFile('goldens/conversation_list.png'),
      );
    });
  });
}
