import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/api/conversation_list_cubit.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/user_cubit.dart';

class ConversationListCubit
    implements StateStreamableSource<ConversationListState> {
  ConversationListCubit({required UserCubit userCubit})
      : _impl = ConversationListCubitBase(userCubit: userCubit.base);

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

  Future<ConversationId> createConnection({required String userName}) =>
      _impl.createConnection(userName: userName);

  Future<ConversationId> createConversation({required String groupName}) =>
      _impl.createConversation(groupName: groupName);
}
