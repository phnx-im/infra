// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/message_list_cubit.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/user_cubit.dart';

class MessageListCubit extends StateStreamableSource<MessageListState> {
  MessageListCubit({
    required UserCubit userCubit,
    required ConversationId conversationId,
  }) : _impl = MessageListCubitBase(
          userCubit: userCubit.impl,
          conversationId: conversationId,
        );

  final MessageListCubitBase _impl;

  @override
  FutureOr<void> close() {
    _impl.close();
  }

  @override
  bool get isClosed => _impl.isClosed;

  @override
  MessageListState get state => _impl.state;

  @override
  Stream<MessageListState> get stream => _impl.stream();
}
