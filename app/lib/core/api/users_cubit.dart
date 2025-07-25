// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.11.1.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:uuid/uuid.dart';
import 'types.dart';
import 'user_cubit.dart';

// These functions are ignored because they are not marked as `pub`: `new`, `new`, `process_notification`, `process`, `set_profile`, `spawn_load_profile`, `spawn`
// These types are ignored because they are neither used by any `pub` functions nor (for structs and enums) marked `#[frb(unignore)]`: `ProfileLoadingTask`, `UsersStateInner`
// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `clone`, `clone`, `fmt`, `fmt`

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<UsersCubitBase>>
abstract class UsersCubitBase implements RustOpaqueInterface {
  Future<void> close();

  bool get isClosed;

  factory UsersCubitBase({required UserCubitBase userCubit}) => RustLib
      .instance
      .api
      .crateApiUsersCubitUsersCubitBaseNew(userCubit: userCubit);

  UsersState get state;

  Stream<UsersState> stream();
}

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<UsersState>>
abstract class UsersState implements RustOpaqueInterface {
  /// Returns the display name of the given user.
  ///
  /// If the user is not specificed, the display name of the logged-in user is returned.
  ///
  /// If the profile is not yet loaded, the default display name is returned and loading of the
  /// profile is spawned in the background.
  String displayName({UiUserId? userId});

  /// Returns the profile of the given user.
  ///
  /// If the user is not specificed, the profile of the logged-in user is returned.
  ///
  /// If the profile is not yet loaded, the default profile is returned and loading is spawned in
  /// the background.
  UiUserProfile profile({UiUserId? userId});

  /// Returns the profile picture of the given user if any is set.
  ///
  /// If the user is not specificed, the profile picture of the logged-in user is returned.
  ///
  /// If the profile is not yet loaded, `null` is returned and loading of the profile is spawned
  /// in the background.
  ImageData? profilePicture({UiUserId? userId});
}
