// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:air/chat_list/chat_list.dart';
import 'package:air/chat_list/chat_list_cubit.dart';
import 'package:air/core/api/chat_details_cubit.dart';
import 'package:air/core/api/chat_list_cubit.dart';
import 'package:air/core/api/navigation_cubit.dart';
import 'package:air/core/api/types.dart';
import 'package:air/l10n/app_localizations.dart';
import 'package:air/navigation/navigation_cubit.dart';
import 'package:air/theme/theme_data.dart';
import 'package:air/user/user.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:path/path.dart' as p;

import '../helpers.dart';
import '../mocks.dart';
import 'content.dart';
import 'product_shot.dart';

/// Large store headline used in product shots.
class ShotTitle extends StatelessWidget {
  const ShotTitle({super.key, required this.text});

  final String text;

  static const _style = TextStyle(
    fontSize: 64,
    fontWeight: FontWeight.w800,
    color: Color.fromARGB(255, 59, 61, 65), // dark grey title
    height: 1.5,
    letterSpacing: -0.5,
  );

  @override
  Widget build(BuildContext context) {
    return DefaultTextStyle.merge(
      style: _style,
      child: Text(
        text,
        maxLines: 2,
        textAlign: TextAlign.center,
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}

void run({required String outputBase}) {
  late MockNavigationCubit navigationCubit;
  late MockChatListCubit chatListCubit;
  late MockUserCubit userCubit;
  late MockUsersCubit contactsCubit;
  late MockChatDetailsCubit chatDetailsCubit;

  setUp(() async {
    navigationCubit = MockNavigationCubit();
    userCubit = MockUserCubit();
    chatListCubit = MockChatListCubit();
    contactsCubit = MockUsersCubit();
    chatDetailsCubit = MockChatDetailsCubit();

    when(() => navigationCubit.state).thenReturn(const NavigationState.home());
    when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
    when(() => contactsCubit.state).thenReturn(
      MockUsersState(
        profiles: [UiUserProfile(userId: 1.userId(), displayName: "alice")],
      ),
    );
    when(
      () => chatDetailsCubit.state,
    ).thenReturn(ChatDetailsState(chat: chats[1], members: [1.userId()]));
  });

  testWidgets('Chat List', (tester) async {
    when(() => navigationCubit.state).thenReturn(
      NavigationState.home(
        home: HomeNavigationState(chatOpen: true, chatId: chats[1].id),
      ),
    );
    when(() => chatListCubit.state).thenReturn(
      ChatListState(
        chats: List.generate(20, (index) => chats[index % chats.length]),
      ),
    );

    // Set surface size to exactly match ProductShot dimensions
    const shotWidth = 1242;
    const shotHeight = 2000; // 2688;

    addTearDown(() {
      tester.view.resetPhysicalSize;
      tester.view.resetDevicePixelRatio;
    });

    tester.view.physicalSize = Size(
      shotWidth.toDouble(),
      shotHeight.toDouble(),
    );
    tester.view.devicePixelRatio = 1.0;

    const shot = ProductShot(
      widthPx: shotWidth,
      heightPx: shotHeight,
      backgroundColor: Color.fromARGB(
        255,
        221,
        227,
        234,
      ), // light metallic grey
      label: 'Private messaging.\nFor everybody.',
      child: ChatListView(scaffold: true),
    );

    Widget buildSubject() => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ChatListCubit>.value(value: chatListCubit),
      ],
      child: Builder(
        builder: (context) {
          return MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: lightTheme,
            themeMode: ThemeMode.light,
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: Material(
              // Note: This is needed because our color scheme is resolved via platform brightness
              // inside the media query, and not via the theme.
              child: MediaQuery(
                data: MediaQuery.of(
                  context,
                ).copyWith(platformBrightness: Brightness.light),
                child: shot,
              ),
            ),
          );
        },
      ),
    );

    await tester.pumpWidget(buildSubject());
    await tester.pump(const Duration(milliseconds: 100));

    await expectLater(
      find.byType(ProductShot),
      matchesGoldenFile(p.join(outputBase, 'chat_list.png')),
    );
  });
}

void main() {
  run(outputBase: "goldens");
}
