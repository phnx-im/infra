// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:air/chat_details/chat_details.dart';
import 'package:air/chat_list/chat_list.dart';
import 'package:air/chat_list/chat_list_cubit.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/app_localizations.dart';
import 'package:air/message_list/message_list.dart';
import 'package:air/navigation/navigation_cubit.dart';
import 'package:air/theme/theme_data.dart';
import 'package:air/user/user.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../helpers.dart';
import '../message_list/message_list_test.dart';
import '../mocks.dart';
import 'content.dart';
import 'product_shot.dart';
import 'product_shot_device.dart';

const androidPhysicalSize = Size(1080, 1920);
const iosPhysicalSize = Size(1260, 2736);

void main() {
  setUpAll(() {
    registerFallbackValue(0.messageId());
    registerFallbackValue(0.userId());
  });

  group('Chat List Product Shots', () {
    const size = Size(1242, 2000);
    const backgroundColor = Color.fromARGB(255, 221, 227, 234);
    const header = 'Easy private messaging.';
    const subheader = 'Every message in Air is end-to-end encrypted.';

    late MockNavigationCubit navigationCubit;
    late MockChatListCubit chatListCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit usersCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      chatListCubit = MockChatListCubit();
      usersCubit = MockUsersCubit();

      when(
        () => navigationCubit.state,
      ).thenReturn(const NavigationState.home());
      when(() => userCubit.state).thenReturn(MockUiUser(id: 10));
      when(() => usersCubit.state).thenReturn(
        MockUsersState(profiles: userProfiles, defaultUserId: ownId),
      );
      when(() => chatListCubit.state).thenReturn(ChatListState(chats: chats));
    });

    Widget buildSubject(ProductShotPlatform platform) => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: usersCubit),
        BlocProvider<ChatListCubit>.value(value: chatListCubit),
      ],
      child: Builder(
        builder: (context) {
          final shot = ProductShot(
            size: size,
            backgroundColor: backgroundColor,
            header: header,
            subheader: subheader,
            device: ProductShotDevices.forPlatform(platform),
            child: const ChatListView(scaffold: true),
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

    testProductShot(
      hostPlatform: 'macos',
      targetPlatform: TargetPlatform.iOS,
      physicalSize: iosPhysicalSize,
      (tester) async {
        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/chat_list.ios.png"),
        );
      },
    );

    testProductShot(
      hostPlatform: 'linux',
      targetPlatform: TargetPlatform.android,
      physicalSize: androidPhysicalSize,
      (tester) async {
        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/chat_list.android.png"),
        );
      },
    );
  });

  group("Private Chat", () {
    const size = Size(1242, 2000);
    const backgroundColor = Color.fromARGB(255, 236, 226, 215);
    const header = 'Connect with friends.';
    const subheader = 'Send messages in private chats.';

    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockChatDetailsCubit chatDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      chatDetailsCubit = MockChatDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();

      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chats[2].id)),
      );
      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(() => chatDetailsCubit.state).thenReturn(
        ChatDetailsState(
          chat: chats[2],
          members: [1.userId(), 2.userId(), 3.userId()],
        ),
      );
      when(
        () => chatDetailsCubit.markAsRead(
          untilMessageId: any(named: "untilMessageId"),
          untilTimestamp: any(named: "untilTimestamp"),
        ),
      ).thenAnswer((_) => Future.value());
      when(
        () => chatDetailsCubit.storeDraft(
          draftMessage: any(named: "draftMessage"),
        ),
      ).thenAnswer((_) async => Future.value());
      when(() => userSettingsCubit.state).thenReturn(const UserSettings());
    });

    Widget buildSubject(ProductShotPlatform platform) => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ChatDetailsCubit>.value(value: chatDetailsCubit),
        BlocProvider<MessageListCubit>.value(value: messageListCubit),
        BlocProvider<UserSettingsCubit>.value(value: userSettingsCubit),
      ],
      child: Builder(
        builder: (context) {
          final shot = ProductShot(
            size: size,
            backgroundColor: backgroundColor,
            header: header,
            subheader: subheader,
            device: ProductShotDevices.forPlatform(platform),
            child: const ChatScreenView(
              createMessageCubit: createMockMessageCubit,
            ),
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

    testProductShot(
      hostPlatform: "macos",
      targetPlatform: TargetPlatform.iOS,
      physicalSize: iosPhysicalSize,
      (tester) async {
        when(
          () => messageListCubit.state,
        ).thenReturn(MockMessageListState(messages));

        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/private_chat.ios.png"),
        );
      },
    );

    testProductShot(
      hostPlatform: "linux",
      targetPlatform: TargetPlatform.android,
      physicalSize: androidPhysicalSize,
      (tester) async {
        when(
          () => messageListCubit.state,
        ).thenReturn(MockMessageListState(messages));

        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/private_chat.android.png"),
        );
      },
    );
  });

  group("Group Chat", () {
    const size = Size(1242, 2000);
    const backgroundColor = Color.fromARGB(255, 219, 231, 217);
    const header = 'Create groups to chat.';
    const subheader = 'Chat in groups with multiple people.';

    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockChatDetailsCubit chatDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      chatDetailsCubit = MockChatDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();

      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chats[2].id)),
      );
      when(() => userCubit.state).thenReturn(MockUiUser(id: 1));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(() => chatDetailsCubit.state).thenReturn(
        ChatDetailsState(
          chat: chats[2],
          members: [1.userId(), 2.userId(), 3.userId()],
        ),
      );
      when(
        () => chatDetailsCubit.markAsRead(
          untilMessageId: any(named: "untilMessageId"),
          untilTimestamp: any(named: "untilTimestamp"),
        ),
      ).thenAnswer((_) => Future.value());
      when(
        () => chatDetailsCubit.storeDraft(
          draftMessage: any(named: "draftMessage"),
        ),
      ).thenAnswer((_) async => Future.value());
      when(() => userSettingsCubit.state).thenReturn(const UserSettings());
    });

    Widget buildSubject(ProductShotPlatform platform) => MultiBlocProvider(
      providers: [
        BlocProvider<NavigationCubit>.value(value: navigationCubit),
        BlocProvider<UserCubit>.value(value: userCubit),
        BlocProvider<UsersCubit>.value(value: contactsCubit),
        BlocProvider<ChatDetailsCubit>.value(value: chatDetailsCubit),
        BlocProvider<MessageListCubit>.value(value: messageListCubit),
        BlocProvider<UserSettingsCubit>.value(value: userSettingsCubit),
      ],
      child: Builder(
        builder: (context) {
          final shot = ProductShot(
            size: size,
            backgroundColor: backgroundColor,
            header: header,
            subheader: subheader,
            device: ProductShotDevices.forPlatform(platform),
            child: const ChatScreenView(
              createMessageCubit: createMockMessageCubit,
            ),
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

    testProductShot(
      hostPlatform: "macos",
      targetPlatform: TargetPlatform.iOS,
      physicalSize: iosPhysicalSize,
      (tester) async {
        when(
          () => messageListCubit.state,
        ).thenReturn(MockMessageListState(messages));

        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/group_chat.ios.png"),
        );
      },
    );

    testProductShot(
      hostPlatform: "linux",
      targetPlatform: TargetPlatform.android,
      physicalSize: androidPhysicalSize,
      (tester) async {
        when(
          () => messageListCubit.state,
        ).thenReturn(MockMessageListState(messages));

        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          matchesGoldenFile("goldens/group_chat.android.png"),
        );
      },
    );
  });
}

void testProductShot(
  WidgetTesterCallback callback, {
  required String hostPlatform,
  required TargetPlatform targetPlatform,
  required Size physicalSize,
}) async {
  testWidgets(targetPlatform.toString(), (tester) async {
    debugDisableShadows = false;
    debugDefaultTargetPlatformOverride = targetPlatform;

    tester.view.physicalSize = androidPhysicalSize;
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    try {
      await callback(tester);
    } finally {
      debugDisableShadows = true;
      debugDefaultTargetPlatformOverride = null;
    }
  }, skip: Platform.operatingSystem != hostPlatform);
}
