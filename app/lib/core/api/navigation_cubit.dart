// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.11.1.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'notifications.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:freezed_annotation/freezed_annotation.dart' hide protected;
import 'package:uuid/uuid.dart';
import 'types.dart';
part 'navigation_cubit.freezed.dart';

// These functions are ignored because they are not marked as `pub`: `home`, `intro`, `subscribe`
// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `clone`, `clone`, `clone`, `clone`, `clone`, `eq`, `eq`, `eq`, `eq`, `eq`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `from`, `from`

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<NavigationCubitBase>>
abstract class NavigationCubitBase implements RustOpaqueInterface {
  Future<void> close();

  Future<void> closeConversation();

  bool get isClosed;

  factory NavigationCubitBase({
    required DartNotificationService notificationService,
  }) => RustLib.instance.api.crateApiNavigationCubitNavigationCubitBaseNew(
    notificationService: notificationService,
  );

  Future<void> openAddMembers();

  Future<void> openConversation({required ConversationId conversationId});

  Future<void> openConversationDetails();

  Future<void> openDeveloperSettings({
    required DeveloperSettingsScreenType screen,
  });

  Future<void> openHome();

  Future<void> openInto();

  Future<void> openIntroScreen({required IntroScreenType screen});

  Future<void> openMemberDetails({required UiUserId member});

  Future<void> openUserSettings({required UserSettingsScreenType screen});

  bool pop();

  NavigationState get state;

  Stream<NavigationState> stream();
}

enum DeveloperSettingsScreenType { root, changeUser, logs }

/// Conversations screen: main screen of the app
///
/// Note: this can be represented in a better way disallowing invalid states.
/// For now, following KISS we represent the navigation stack in a very simple
/// way by just storing true/false or an optional value representing if a
/// screen is opened.
@freezed
sealed class HomeNavigationState with _$HomeNavigationState {
  const HomeNavigationState._();
  const factory HomeNavigationState({
    @Default(false) bool conversationOpen,
    ConversationId? conversationId,
    DeveloperSettingsScreenType? developerSettingsScreen,
    UiUserId? memberDetails,
    UserSettingsScreenType? userSettingsScreen,
    @Default(false) bool conversationDetailsOpen,
    @Default(false) bool addMembersOpen,
  }) = _HomeNavigationState;
  static Future<HomeNavigationState> default_() =>
      RustLib.instance.api.crateApiNavigationCubitHomeNavigationStateDefault();
}

@freezed
sealed class IntroScreenType with _$IntroScreenType {
  const IntroScreenType._();

  const factory IntroScreenType.intro() = IntroScreenType_Intro;
  const factory IntroScreenType.serverChoice() = IntroScreenType_ServerChoice;
  const factory IntroScreenType.displayNamePicture() =
      IntroScreenType_DisplayNamePicture;
  const factory IntroScreenType.developerSettings(
    DeveloperSettingsScreenType field0,
  ) = IntroScreenType_DeveloperSettings;
}

@freezed
sealed class NavigationState with _$NavigationState {
  const NavigationState._();

  /// Intro screen: welcome and registration screen
  const factory NavigationState.intro({
    @Default([]) List<IntroScreenType> screens,
  }) = NavigationState_Intro;
  const factory NavigationState.home({
    @Default(HomeNavigationState()) HomeNavigationState home,
  }) = NavigationState_Home;
}

enum UserSettingsScreenType { root, editDisplayName, addUserHandle }
