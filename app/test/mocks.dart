// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:bloc_test/bloc_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/message_list/message_cubit.dart';
import 'package:prototype/message_list/message_list_cubit.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/user/user.dart';

class MockNavigationCubit extends MockCubit<NavigationState>
    implements NavigationCubit {}

class MockUserCubit extends MockCubit<UiUser> implements UserCubit {}

class MockUiUser implements UiUser {
  MockUiUser({
    required String userName,
    String? displayName,
    Uint8List? profilePicture,
  })  : _userName = userName,
        _displayName = displayName,
        _profilePicture = profilePicture;

  final String _userName;
  final String? _displayName;
  final Uint8List? _profilePicture;

  @override
  String? get displayName => _displayName;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  Uint8List? get profilePicture => _profilePicture;

  @override
  String get userName => _userName;
}

class MockConversationDetailsCubit extends MockCubit<ConversationDetailsState>
    implements ConversationDetailsCubit {}

class MockConversationListState extends Mock implements ConversationListState {}

class MockConversationListCubit extends MockCubit<ConversationListState>
    implements ConversationListCubit {}

class MockMessageListCubit extends MockCubit<MessageListState>
    implements MessageListCubit {}

class MockMessageListState implements MessageListState {
  MockMessageListState(this.messages);

  final List<UiConversationMessage> messages;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  int get loadedMessagesCount => messages.length;

  @override
  UiConversationMessage? messageAt(int index) =>
      messages.elementAtOrNull(index);

  @override
  int? messageIdIndex(ConversationMessageId messageId) {
    final index = messages.indexWhere((element) => element.id == messageId);
    return index != -1 ? index : null;
  }
}

class MockMessageCubit extends MockCubit<MessageState> implements MessageCubit {
  MockMessageCubit({required MessageState initialState}) {
    when(() => state).thenReturn(initialState);
  }
}
