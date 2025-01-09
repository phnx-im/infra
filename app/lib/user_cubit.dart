// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/core_client.dart';

/// Wrapper of the [UserCubitBase] that implements a [StateStreamableSource]
///
// See <https://github.com/phnx-im/infra/issues/248>
class UserCubit implements StateStreamableSource<UiUser> {
  UserCubit({required CoreClient coreClient})
      : _impl = UserCubitBase(user: coreClient.user);

  final UserCubitBase _impl;

  UserCubitBase get impl => _impl;

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

  Future<UiUserProfile?> userProfile(String userName) =>
      _impl.userProfile(userName);

  Future<void> addUserToConversation(
    ConversationId conversationId,
    String userName,
  ) =>
      _impl.addUserToConversation(conversationId, userName);

  Future<void> removeUserFromConversation(
    ConversationId conversationId,
    String userName,
  ) =>
      _impl.removeUserFromConversation(conversationId, userName);

  Future<List<UiContact>> get contacts => _impl.contacts;
}
