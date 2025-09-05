// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:air/conversation_details/conversation_details.dart';
import 'package:air/conversation_details/group_details.dart';
import 'package:air/core/core.dart';
import 'package:air/user/user.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/theme/theme.dart';
import 'package:mocktail/mocktail.dart';

import '../conversation_list/conversation_list_content_test.dart';
import '../mocks.dart';

void main() {
  group('GroupDetails', () {
    late MockConversationDetailsCubit conversationDetailsCubit;
    late MockUsersCubit usersCubit;

    setUp(() async {
      conversationDetailsCubit = MockConversationDetailsCubit();
      usersCubit = MockUsersCubit();

      when(
        () => usersCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
    });

    Widget buildSubject({List<UiUserId> members = const []}) {
      when(() => conversationDetailsCubit.state).thenReturn(
        ConversationDetailsState(
          conversation: conversations[2],
          members: members,
        ),
      );

      return MultiBlocProvider(
        providers: [
          BlocProvider<ConversationDetailsCubit>.value(
            value: conversationDetailsCubit,
          ),
          BlocProvider<UsersCubit>.value(value: usersCubit),
        ],
        child: MaterialApp(
          debugShowCheckedModeBanner: false,
          theme: lightTheme,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          home: const Scaffold(body: GroupDetails()),
        ),
      );
    }

    testWidgets('renders correctly', (tester) async {
      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/group_details.png'),
      );
    });

    testWidgets('renders correctly with members overflowing', (tester) async {
      final members =
          (userProfiles + userProfiles + userProfiles)
              .map((e) => e.userId)
              .toList();
      await tester.pumpWidget(buildSubject(members: members));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/group_details_members_overflow.png'),
      );
    });

    testWidgets('renders correctly empty', (tester) async {
      await tester.pumpWidget(buildSubject(members: []));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/group_details_empty.png'),
      );
    });
  });
}
