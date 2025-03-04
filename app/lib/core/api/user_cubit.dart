// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.7.1.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:uuid/uuid.dart';
import 'types.dart';
import 'user.dart';

// These functions are ignored because they are not marked as `pub`: `emit`, `handle_websocket_message`, `new`, `process_fetched_messages`, `run_websocket`, `spawn_load`, `spawn_polling`, `spawn_websocket`
// These types are ignored because they are not used by any `pub` functions: `UiUserInner`
// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `clone`, `drop`, `fmt`, `fmt`

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<UiUser>>
abstract class UiUser implements RustOpaqueInterface {
  String? get displayName;

  ImageData? get profilePicture;

  String get userName;
}

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<UserCubitBase>>
abstract class UserCubitBase implements RustOpaqueInterface {
  Future<void> addUserToConversation(
      ConversationId conversationId, String userName);

  Future<void> close();

  Future<List<UiContact>> get contacts;

  bool get isClosed;

  factory UserCubitBase({required User user}) =>
      RustLib.instance.api.crateApiUserCubitUserCubitBaseNew(user: user);

  Future<void> removeUserFromConversation(
      ConversationId conversationId, String userName);

  /// Set the display name and/or profile picture of the user.
  Future<void> setProfile({String? displayName, Uint8List? profilePicture});

  UiUser get state;

  Stream<UiUser> stream();

  /// Get the user profile of the user with the given [`QualifiedUserName`].
  Future<UiUserProfile?> userProfile(String userName);
}
