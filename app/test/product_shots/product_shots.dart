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
import 'package:flutter/rendering.dart' show debugDisableShadows;
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:path/path.dart' as p;

import '../helpers.dart';
import '../mocks.dart';
import 'content.dart';
import 'product_shot.dart';
import 'product_shot_device.dart';

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

  setUpAll(() {
    // Enable real blurred shadows in widget tests (disabled by default).
    debugDisableShadows = false;
  });

  tearDownAll(() {
    // Restore default behavior for other tests.
    debugDisableShadows = true;
  });

  for (final platform in _platformsUnderTest) {
    for (final scenario in _scenarios) {
      final goldenFile = p.join(
        outputBase,
        '${scenario.id}.${_platformSlug(platform)}.png',
      );
      final description = '${_platformTitle(platform)} - ${scenario.id}';

      testWidgets(description, (tester) async {
        _primeChatListCubit(chatListCubit);
        _primeNavigation(navigationCubit);

        const shotWidth = 1242;
        const shotHeight = 2000;

        addTearDown(() {
          tester.view.resetPhysicalSize();
          tester.view.resetDevicePixelRatio();
        });

        tester.view.physicalSize = const Size(1242, 2000);
        tester.view.devicePixelRatio = 1.0;

        Widget buildSubject() => MultiBlocProvider(
          providers: [
            BlocProvider<NavigationCubit>.value(value: navigationCubit),
            BlocProvider<UserCubit>.value(value: userCubit),
            BlocProvider<UsersCubit>.value(value: contactsCubit),
            BlocProvider<ChatListCubit>.value(value: chatListCubit),
          ],
          child: Builder(
            builder: (context) {
              final shot = ProductShot(
                widthPx: shotWidth,
                heightPx: shotHeight,
                backgroundColor: scenario.backgroundColor,
                label: scenario.label(platform),
                device: ProductShotDevices.forPlatform(platform),
                child: scenario.childBuilder(context),
              );

              return MaterialApp(
                debugShowCheckedModeBanner: false,
                theme: lightTheme,
                themeMode: ThemeMode.light,
                localizationsDelegates: AppLocalizations.localizationsDelegates,
                home: Material(
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
          matchesGoldenFile(goldenFile),
        );
      });
    }
  }
}

void _primeChatListCubit(MockChatListCubit chatListCubit) {
  when(() => chatListCubit.state).thenReturn(
    ChatListState(
      chats: List.generate(20, (index) => chats[index % chats.length]),
    ),
  );
}

void _primeNavigation(MockNavigationCubit navigationCubit) {
  when(() => navigationCubit.state).thenReturn(
    NavigationState.home(
      home: HomeNavigationState(chatOpen: true, chatId: chats[1].id),
    ),
  );
}

const List<ProductShotPlatform> _platformsUnderTest = <ProductShotPlatform>[
  ProductShotPlatform.android,
  ProductShotPlatform.ios,
  ProductShotPlatform.macos,
];

typedef _ShotContentBuilder = Widget Function(BuildContext context);

class _ShotScenario {
  const _ShotScenario({
    required this.id,
    required this.backgroundColor,
    required this.labelBuilder,
    required this.childBuilder,
  });

  final String id;
  final Color backgroundColor;
  final String Function(ProductShotPlatform platform) labelBuilder;
  final _ShotContentBuilder childBuilder;

  String label(ProductShotPlatform platform) => labelBuilder(platform);
}

String _syncTagline(ProductShotPlatform platform) {
  final target = _platformMarketingName(platform);
  return 'Always in sync on $target.';
}

String _privacyTagline(ProductShotPlatform platform) {
  final target = _platformMarketingName(platform);
  return 'Private messaging on $target.';
}

String _groupsTagline(ProductShotPlatform platform) {
  final target = _platformMarketingName(platform);
  return 'Groups that feel native on $target.';
}

final List<_ShotScenario> _scenarios = <_ShotScenario>[
  _ShotScenario(
    id: 'chat_list_primary',
    backgroundColor: const Color.fromARGB(255, 221, 227, 234),
    labelBuilder:
        (platform) => 'Private messaging.\n${_privacyTagline(platform)}',
    childBuilder: _buildChatListView,
  ),
  _ShotScenario(
    id: 'chat_list_groups',
    backgroundColor: const Color.fromARGB(255, 219, 231, 217),
    labelBuilder: (platform) => 'Group catch-ups.\n${_groupsTagline(platform)}',
    childBuilder: _buildChatListView,
  ),
  _ShotScenario(
    id: 'chat_list_sync',
    backgroundColor: const Color.fromARGB(255, 236, 226, 215),
    labelBuilder: (platform) => 'Stay in sync.\n${_syncTagline(platform)}',
    childBuilder: _buildChatListView,
  ),
];

Widget _buildChatListView(BuildContext context) {
  return ChatListView(scaffold: true);
}

String _platformSlug(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return 'android';
    case ProductShotPlatform.ios:
      return 'ios';
    case ProductShotPlatform.macos:
      return 'macos';
    case ProductShotPlatform.windows:
      return 'windows';
    case ProductShotPlatform.linux:
      return 'linux';
  }
}

String _platformTitle(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return 'Android';
    case ProductShotPlatform.ios:
      return 'iOS';
    case ProductShotPlatform.macos:
      return 'macOS';
    case ProductShotPlatform.windows:
      return 'Windows';
    case ProductShotPlatform.linux:
      return 'Linux';
  }
}

String _platformMarketingName(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return 'Android';
    case ProductShotPlatform.ios:
      return 'iPhone';
    case ProductShotPlatform.macos:
      return 'Mac';
    case ProductShotPlatform.windows:
      return 'Windows';
    case ProductShotPlatform.linux:
      return 'Linux';
  }
}

void main() {
  run(outputBase: "goldens");
}
