// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';

/// Wrapper of the [UserCubitBase] that implements a [StateStreamableSource]
///
// See <https://github.com/phnx-im/infra/issues/248>
class UserCubit implements StateStreamableSource<UiUser> {
  UserCubit({
    required CoreClient coreClient,
    required NavigationCubit navigationCubit,
    required Stream<AppState> appStateStream,
  }) : _impl = UserCubitBase(
         user: coreClient.user,
         navigation: navigationCubit.base,
       ) {
    _appStateSubscription = appStateStream.listen(
      (appState) => _impl.setAppState(appState: appState),
    );
  }

  final UserCubitBase _impl;
  late final StreamSubscription<AppState> _appStateSubscription;

  UserCubitBase get impl => _impl;

  @override
  FutureOr<void> close() {
    _appStateSubscription.cancel();
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

  Future<void> addUserToConversation(
    ConversationId conversationId,
    UiUserId userId,
  ) => _impl.addUserToConversation(conversationId, userId);

  Future<void> removeUserFromConversation(
    ConversationId conversationId,
    UiUserId userId,
  ) => _impl.removeUserFromConversation(conversationId, userId);

  Future<void> leaveConversation(ConversationId conversationId) =>
      _impl.leaveConversation(conversationId);

  Future<void> deleteConversation(ConversationId conversationId) =>
      _impl.deleteConversation(conversationId);

  Future<List<UiContact>> get contacts => _impl.contacts;

  Future<bool> addUserHandle(UiUserHandle userHandle) =>
      _impl.addUserHandle(userHandle: userHandle);

  Future<void> removeUserHandle(UiUserHandle userHandle) =>
      _impl.removeUserHandle(userHandle: userHandle);

  Future<List<UiContact>> addableContacts(ConversationId conversationId) =>
      _impl.addableContacts(conversationId: conversationId);
}
