// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/core/core.dart';
import 'package:air/user/user.dart';

class ConversationListCubit
    implements StateStreamableSource<ConversationListState> {
  ConversationListCubit({required UserCubit userCubit})
    : _impl = ConversationListCubitBase(userCubit: userCubit.impl);

  final ConversationListCubitBase _impl;

  @override
  FutureOr<void> close() {
    _impl.close();
  }

  @override
  bool get isClosed => _impl.isClosed;

  @override
  ConversationListState get state => _impl.state;

  @override
  Stream<ConversationListState> get stream => _impl.stream();

  Future<ConversationId?> createConnection({required UiUserHandle handle}) =>
      _impl.createConnection(handle: handle);

  Future<ConversationId> createConversation({required String groupName}) =>
      _impl.createConversation(groupName: groupName);
}
