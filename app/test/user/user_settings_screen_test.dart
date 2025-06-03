// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';

import '../helpers.dart';
import '../mocks.dart';

void main() {
  group('UserSettingsScreenTest', () {
    late MockUserCubit userCubit;
    late MockContactsCubit contactsCubit;

    setUp(() async {
      userCubit = MockUserCubit();
      contactsCubit = MockContactsCubit();

      when(() => contactsCubit.state).thenReturn(
        MockContactsState(
          profiles: [UiUserProfile(userId: 1.userId(), displayName: "alice")],
        ),
      );
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<ContactsCubit>.value(value: contactsCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: themeData(context),
            home: const UserSettingsScreen(),
          );
        },
      ),
    );

    testWidgets('renders correctly (no handles)', (tester) async {
      when(
        () => userCubit.state,
      ).thenReturn(MockUiUser(id: 1, userHandles: []));

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/user_settings_screen_no_handles.png'),
      );
    });

    testWidgets('renders correctly (some handles)', (tester) async {
      when(() => userCubit.state).thenReturn(
        MockUiUser(
          id: 1,
          userHandles: [
            const UiUserHandle(plaintext: "ellie"),
            const UiUserHandle(plaintext: "firefly"),
          ],
        ),
      );

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/user_settings_screen_some_handles.png'),
      );
    });

    testWidgets('renders correctly (all handles)', (tester) async {
      when(() => userCubit.state).thenReturn(
        MockUiUser(
          id: 1,
          userHandles: [
            const UiUserHandle(plaintext: "ellie"),
            const UiUserHandle(plaintext: "firefly"),
            const UiUserHandle(plaintext: "kiddo"),
            const UiUserHandle(plaintext: "ells"),
            const UiUserHandle(plaintext: "wolf"),
          ],
        ),
      );

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/user_settings_screen_all_handles.png'),
      );
    });
  });
}
