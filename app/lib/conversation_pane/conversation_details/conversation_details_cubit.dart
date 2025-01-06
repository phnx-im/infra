// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/conversation_details_cubit.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/user_cubit.dart';

class ConversationDetailsCubit
    extends StateStreamableSource<ConversationDetailsState> {
  ConversationDetailsCubit({
    required UserCubit userCubit,
    required ConversationId conversationId,
  }) : _impl = ConversationDetailsCubitBase(
          userCubit: userCubit.impl,
          conversationId: conversationId,
        );

  final ConversationDetailsCubitBase _impl;

  @override
  FutureOr<void> close() {
    _impl.close();
  }

  @override
  bool get isClosed => _impl.isClosed;

  @override
  ConversationDetailsState get state => _impl.state;

  @override
  Stream<ConversationDetailsState> get stream => _impl.stream();

  // Cubit methods

  Future<void> setConversationPicture({required Uint8List? bytes}) =>
      _impl.setConversationPicture(bytes: bytes);

  Future<UiUserProfile?> loadConversationUserProfile() =>
      _impl.loadConversationUserProfile();

  UiConversationMessageId? messageIdFromRevOffset(int offset) =>
      _impl.messageIdFromRevOffset(offset: offset);

  int? revOffsetFromMessageId(UiConversationMessageId messageId) =>
      _impl.revOffsetFromMessageId(messageId: messageId);
}
