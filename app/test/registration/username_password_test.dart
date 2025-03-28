// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/registration/registration.dart';
import 'package:prototype/theme/theme.dart';

import '../mocks.dart';

void main() {
  group('UsernamePasswordChoice', () {
    late MockRegistrationCubit registrationCubit;

    setUp(() async {
      registrationCubit = MockRegistrationCubit();
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<RegistrationCubit>.value(value: registrationCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: themeData(context),
            home: const Scaffold(body: UsernamePasswordChoice()),
          );
        },
      ),
    );

    testWidgets('renders correctly when empty', (tester) async {
      when(() => registrationCubit.state).thenReturn(const RegistrationState());

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/username_password_choice_empty.png'),
      );
    });

    testWidgets('renders correctly', (tester) async {
      when(() => registrationCubit.state).thenReturn(
        const RegistrationState(
          username: "alice",
          password: "test",
          isUsernameValid: true,
          isPasswordValid: true,
        ),
      );

      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/username_password_choice.png'),
      );
    });
  });
}
