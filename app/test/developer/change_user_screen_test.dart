// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/developer/developer.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';

import '../helpers.dart';
import '../mocks.dart';

final clientRecords = [
  UiClientRecord(
    clientId: 1.clientId(),
    createdAt: DateTime.parse("2023-01-01T00:00:00.000Z"),
    userProfile: UiUserProfile(clientId: 1.clientId(), displayName: "alice"),
    isFinished: true,
  ),
  UiClientRecord(
    clientId: 2.clientId(),
    createdAt: DateTime.parse("2024-01-01T00:00:00.000Z"),
    userProfile: UiUserProfile(clientId: 2.clientId(), displayName: "alice"),
    isFinished: true,
  ),
  UiClientRecord(
    clientId: 3.clientId(),
    createdAt: DateTime.parse("2025-01-01T00:00:00.000Z"),
    userProfile: UiUserProfile(clientId: 3.clientId(), displayName: "bob"),
    isFinished: false,
  ),
];

void main() {
  group('DeveloperSettingsScreen', () {
    late MockUser user;
    late MockLoadableUserCubit loadableUserCubit;

    setUp(() async {
      user = MockUser();
      loadableUserCubit = MockLoadableUserCubit();

      when(() => user.clientId).thenReturn(1.clientId());
      when(() => loadableUserCubit.state).thenReturn(LoadableUser.loaded(user));
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<LoadableUserCubit>.value(value: loadableUserCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: themeData(context),
            home: ChangeUserScreenView(
              clientRecords: Future.value(clientRecords),
            ),
          );
        },
      ),
    );

    testWidgets('renders correctly', (tester) async {
      await tester.pumpWidget(buildSubject());
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/change_user_screen.png'),
      );
    });
  });
}
