// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/user/user_cubit.dart';
import 'package:prototype/core_client.dart';

class UserCubit implements StateStreamableSource<UiUser> {
  UserCubit({required CoreClient coreClient})
      : _impl = UserCubitBase(user: coreClient.user);

  final UserCubitBase _impl;

  @override
  FutureOr<void> close() {
    _impl.close();
  }

  @override
  bool get isClosed => _impl.isClosed;

  @override
  UiUser get state => _impl.state;

  @override
  Stream<UiUser> get stream => _impl.stream();

  // Cubit methods

  Future<void> setProfile({String? displayName, Uint8List? profilePicture}) =>
      _impl.setProfile(
        displayName: displayName,
        profilePicture: profilePicture,
      );
}
