// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:bloc_test/bloc_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/conversation_list/conversation_list_cubit.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/message_list/message_cubit.dart';
import 'package:prototype/message_list/message_list_cubit.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/registration/registration.dart';
import 'package:prototype/user/user.dart';

import 'helpers.dart';

class MockNavigationCubit extends MockCubit<NavigationState>
    implements NavigationCubit {}

class MockUserCubit extends MockCubit<UiUser> implements UserCubit {}

class MockUsersCubit extends MockCubit<UsersState> implements UsersCubit {}

class MockUiUser implements UiUser {
  MockUiUser({required int id, List<UiUserHandle> userHandles = const []})
    : _userId = id.userId(),
      _userHandles = userHandles;

  final UiUserId _userId;
  final List<UiUserHandle> _userHandles;

  @override
  UiUserId get userId => _userId;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  List<UiUserHandle> get userHandles => _userHandles;
}

class MockUsersState implements UsersState {
  MockUsersState({
    UiUserId? defaultUserId,
    required List<UiUserProfile> profiles,
  }) : _defaultUserId = defaultUserId ?? 1.userId(),
       _profiles = {for (final profile in profiles) profile.userId: profile};

  final UiUserId _defaultUserId;
  final Map<UiUserId, UiUserProfile> _profiles;

  @override
  UiUserProfile profile({UiUserId? userId}) {
    final id = userId ?? _defaultUserId;
    return _profiles[id]!;
  }

  @override
  String displayName({UiUserId? userId}) => profile(userId: userId).displayName;

  @override
  ImageData? profilePicture({UiUserId? userId}) =>
      profile(userId: userId).profilePicture;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;
}

class MockConversationDetailsCubit extends MockCubit<ConversationDetailsState>
    implements ConversationDetailsCubit {}

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

class MockLoadableUserCubit extends MockCubit<LoadableUser>
    implements LoadableUserCubit {}

class MockUser extends Mock implements User {}

class MockRegistrationCubit extends MockCubit<RegistrationState>
    implements RegistrationCubit {}

class MockAttachmentsRepository extends Mock implements AttachmentsRepository {}
