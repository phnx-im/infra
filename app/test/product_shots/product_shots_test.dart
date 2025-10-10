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

const androidPhysicalSize = Size(2160, 3840);
const iosPhysicalSize = Size(1290, 2796);

const androidProductShotSize = Size(2160, 3840);
const iosProductShotSize = Size(1290, 2796);
const _defaultProductShotSize = Size(1242, 2000);

Size _productShotSizeFor(ProductShotPlatform platform) {
  switch (platform) {
    case ProductShotPlatform.android:
      return androidProductShotSize;
    case ProductShotPlatform.ios:
      return iosProductShotSize;
    case ProductShotPlatform.macos:
    case ProductShotPlatform.windows:
    case ProductShotPlatform.linux:
      return _defaultProductShotSize;
  }
}

void main() {
  setUpAll(() {
    registerFallbackValue(0.messageId());
    registerFallbackValue(0.userId());
  });

  group('Chat List Product Shots', () {
    const backgroundColor = Color.fromARGB(255, 221, 227, 234);
    const header = 'Easy private messaging.';
    const subheader = 'Every message encrypted.';

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
          final shotSize = _productShotSizeFor(platform);
          final shot = ProductShot(
            size: shotSize,
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
      "Chat List (iOS)",
      hostPlatform: 'macos',
      physicalSize: iosPhysicalSize,
      (tester) async {
        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/ios/en-US/screenshots
          matchesGoldenFile("goldens/chat_list.ios.png"),
        );
      },
    );

    testProductShot(
      "Chat List (Android)",
      hostPlatform: 'linux',
      physicalSize: androidPhysicalSize,
      (tester) async {
        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/android/metadata/en-US/images/phone-screenshots
          matchesGoldenFile("goldens/chat_list.android.png"),
        );
      },
    );
  });

  group("Private Chat", () {
    const backgroundColor = Color.fromARGB(255, 236, 226, 215);
    const header = 'Connect with friends.';
    const subheader = 'Send messages in private chats.';

    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockChatDetailsCubit chatDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;
    late MockAttachmentsRepository attachmentsRepository;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      chatDetailsCubit = MockChatDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();
      attachmentsRepository = MockAttachmentsRepository();

      final chat = chats[0];

      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chat.id)),
      );
      when(() => userCubit.state).thenReturn(MockUiUser(id: ownIdx));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(
        () => chatDetailsCubit.state,
      ).thenReturn(ChatDetailsState(chat: chat, members: [fredId]));
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
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(fredMessages));
    });

    Widget buildSubject(ProductShotPlatform platform) =>
        RepositoryProvider<AttachmentsRepository>.value(
          value: attachmentsRepository,
          child: MultiBlocProvider(
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
                final shotSize = _productShotSizeFor(platform);
                final shot = ProductShot(
                  size: shotSize,
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
                  localizationsDelegates:
                      AppLocalizations.localizationsDelegates,
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
          ),
        );

    testProductShot(
      "Private Chat (iOS)",
      hostPlatform: "macos",
      physicalSize: iosPhysicalSize,
      (tester) async {
        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/ios/en-US/screenshots
          matchesGoldenFile("goldens/private_chat.ios.png"),
        );
      },
    );

    testProductShot(
      "Private Chat (Android)",
      hostPlatform: "linux",
      physicalSize: androidPhysicalSize,
      (tester) async {
        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/android/metadata/en-US/screenshots
          matchesGoldenFile("goldens/private_chat.android.png"),
        );
      },
    );
  });

  group("Group Chat", () {
    const backgroundColor = Color.fromARGB(255, 219, 231, 217);
    const header = 'Create groups to chat.';
    const subheader = 'Chat in groups with multiple people.';

    late MockNavigationCubit navigationCubit;
    late MockUserCubit userCubit;
    late MockUsersCubit contactsCubit;
    late MockChatDetailsCubit chatDetailsCubit;
    late MockMessageListCubit messageListCubit;
    late MockUserSettingsCubit userSettingsCubit;
    late MockAttachmentsRepository attachmentsRepository;

    setUp(() async {
      navigationCubit = MockNavigationCubit();
      userCubit = MockUserCubit();
      contactsCubit = MockUsersCubit();
      chatDetailsCubit = MockChatDetailsCubit();
      messageListCubit = MockMessageListCubit();
      userSettingsCubit = MockUserSettingsCubit();
      attachmentsRepository = MockAttachmentsRepository();

      final chat = chats[4];

      when(() => navigationCubit.state).thenReturn(
        NavigationState.home(home: HomeNavigationState(chatId: chat.id)),
      );
      when(() => userCubit.state).thenReturn(MockUiUser(id: ownIdx));
      when(
        () => contactsCubit.state,
      ).thenReturn(MockUsersState(profiles: userProfiles));
      when(() => chatDetailsCubit.state).thenReturn(
        ChatDetailsState(chat: chat, members: gardeningPartyMembers),
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
      when(
        () => messageListCubit.state,
      ).thenReturn(MockMessageListState(gardeningPartyMessages));
    });

    Widget buildSubject(ProductShotPlatform platform) =>
        RepositoryProvider<AttachmentsRepository>.value(
          value: attachmentsRepository,
          child: MultiBlocProvider(
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
                final shotSize = _productShotSizeFor(platform);
                final shot = ProductShot(
                  size: shotSize,
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
                  localizationsDelegates:
                      AppLocalizations.localizationsDelegates,
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
          ),
        );

    testProductShot(
      "Group Chat (iOS)",
      hostPlatform: "macos",
      physicalSize: iosPhysicalSize,
      (tester) async {
        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.ios));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/ios/en-US/screenshots
          matchesGoldenFile("goldens/group_chat.ios.png"),
        );
      },
    );

    testProductShot(
      "Group Chat (Android)",
      hostPlatform: "linux",
      physicalSize: androidPhysicalSize,
      (tester) async {
        VisibilityDetectorController.instance.updateInterval = Duration.zero;

        await tester.pumpWidget(buildSubject(ProductShotPlatform.android));
        await expectLater(
          find.byType(ProductShot),
          // Do not change the file name, as it is referenced in stores/android/metadata/en-US/screenshots
          matchesGoldenFile("goldens/group_chat.android.png"),
        );
      },
    );
  });
}

void testProductShot(
  String description,
  WidgetTesterCallback callback, {
  required String hostPlatform,
  required Size physicalSize,
}) async {
  testWidgets(description, (tester) async {
    debugDisableShadows = false;

    tester.view.physicalSize = physicalSize;
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    try {
      await callback(tester);
    } finally {
      debugDisableShadows = true;
    }
  }, skip: Platform.operatingSystem != hostPlatform);
}
