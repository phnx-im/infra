// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:air/conversation_details/connection_details.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:air/conversation_details/conversation_details.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/theme/theme.dart';

import '../conversation_list/conversation_list_content_test.dart';
import '../mocks.dart';

final chat = chats[0];

void main() {
  group('ConnectionDetailsTest', () {
    late MockConversationDetailsCubit conversationDetailsCubit;

    setUp(() async {
      conversationDetailsCubit = MockConversationDetailsCubit();

      when(() => conversationDetailsCubit.state).thenReturn(
        ConversationDetailsState(chat: chat, members: [userProfiles[1].userId]),
      );
    });

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<ConversationDetailsCubit>.value(
          value: conversationDetailsCubit,
        ),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: lightTheme,
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: const Scaffold(body: ConnectionDetails()),
          );
        },
      ),
    );

    testWidgets('renders correctly', (tester) async {
      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/connection_details.png'),
      );
    });
  });
}
