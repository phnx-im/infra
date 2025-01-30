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
import 'package:uuid/uuid.dart';

import '../mocks.dart';

final clientRecords = [
  UiClientRecord(
    clientId: UuidValue.fromString("7c19e63f-b636-4808-a034-0b7cdb462bce"),
    userName: UiUserName(
      userName: "alice",
      domain: "localhost",
    ),
    createdAt: DateTime.parse("2023-01-01T00:00:00.000Z"),
    userProfile: null,
  ),
  UiClientRecord(
    clientId: UuidValue.fromString("b984c959-c83f-4c99-8999-e6d9d485b172"),
    userName: UiUserName(
      userName: "alice",
      domain: "example.com",
    ),
    createdAt: DateTime.parse("2024-01-01T00:00:00.000Z"),
    userProfile: null,
  ),
  UiClientRecord(
    clientId: UuidValue.fromString("c5091f2f-9409-41b1-9965-5955d12f39b2"),
    userName: UiUserName(
      userName: "bob",
      domain: "localhost",
    ),
    createdAt: DateTime.parse("2025-01-01T00:00:00.000Z"),
    userProfile: null,
  ),
];

void main() {
  group('DeveloperSettingsScreen', () {
    late MockUser user;
    late MockLoadableUserCubit loadableUserCubit;

    setUp(() async {
      user = MockUser();
      loadableUserCubit = MockLoadableUserCubit();

      when(() => user.userName).thenReturn("alice@localhost");
      // when(() => user.clientId).thenReturn(
      //     UuidValue.fromString("7c19e63f-b636-4808-a034-0b7cdb462bce"));
      when(() => loadableUserCubit.state).thenReturn(LoadableUser.loaded(user));
    });

    Widget buildSubject() => MultiBlocProvider(
          providers: [
            BlocProvider<LoadableUserCubit>.value(
              value: loadableUserCubit,
            ),
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
