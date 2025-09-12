// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/core/core.dart';
import 'package:air/user/user.dart';

class ConversationDetailsCubit
    extends StateStreamableSource<ConversationDetailsState> {
  ConversationDetailsCubit({
    required UserCubit userCubit,
    required ChatId chatId,
  }) : _impl = ConversationDetailsCubitBase(
         userCubit: userCubit.impl,
         chatId: chatId,
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

  Future<void> setChatPicture({required Uint8List? bytes}) =>
      _impl.setChatPicture(bytes: bytes);

  Future<void> sendMessage(String messageText) =>
      _impl.sendMessage(messageText: messageText);

  Future<void> uploadAttachment(String path) =>
      _impl.uploadAttachment(path: path);

  Future<void> markAsRead({
    required MessageId untilMessageId,
    required DateTime untilTimestamp,
  }) => _impl.markAsRead(
    untilMessageId: untilMessageId,
    untilTimestamp: untilTimestamp,
  );

  Future<void> storeDraft({required String draftMessage}) =>
      _impl.storeDraft(draftMessage: draftMessage);

  Future<void> resetDraft() => _impl.resetDraft();

  Future<void> editMessage({MessageId? messageId}) =>
      _impl.editMessage(messageId: messageId);
}
