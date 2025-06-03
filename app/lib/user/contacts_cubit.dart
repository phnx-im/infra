// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/user/user.dart';

/// Repository of all user profiles including the logged in user.
class ContactsCubit implements StateStreamableSource<ContactsState> {
  ContactsCubit({required UserCubit userCubit})
    : _impl = ContactsCubitBase(userCubit: userCubit.impl);

  final ContactsCubitBase _impl;

  @override
  FutureOr<void> close() => _impl.close();

  @override
  bool get isClosed => _impl.isClosed;

  @override
  ContactsState get state => _impl.state;

  @override
  Stream<ContactsState> get stream => _impl.stream();

  // Cubit methods

  /// Returns the profile of the given user.
  ///
  /// If the user is not specificed, the profile of the logged in user is returned.
  ///
  /// If the profile is not yet loaded, the default profile is returned and loading is spawned in
  /// the background.
  UiUserProfile profile({UiUserId? userId}) => _impl.profile(userId: userId);

  /// Returns the display name of the given user.
  ///
  /// If the user is not specificed, the display name of the logged in user is returned.
  ///
  /// If the profile is not yet loaded, the default display name is returned and loading of
  /// the profile is spawned in the background.
  String displayName({UiUserId? userId}) => _impl.displayName(userId: userId);

  /// Returns the profile picture of the given user if any is set.
  ///
  /// If the user is not specificed, the profile picture of the logged in user is returned.
  ///
  /// If the profile is not yet loaded, `null` is returned and loading of
  /// the profile is spawned in the background.
  ImageData? profilePicture({UiUserId? userId}) =>
      _impl.profilePicture(userId: userId);
}
