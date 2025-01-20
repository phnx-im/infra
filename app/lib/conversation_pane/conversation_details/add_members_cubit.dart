// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:collection';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';

part 'add_members_cubit.freezed.dart';

@freezed
class AddMembersState with _$AddMembersState {
  const factory AddMembersState({
    required List<UiContact> contacts,
    required Set<String> selectedContacts,
  }) = _AddMembersState;
}

class AddMembersCubit extends Cubit<AddMembersState> {
  AddMembersCubit({
    required CoreClient coreClient,
  })  : _coreClient = coreClient,
        super(
          const AddMembersState(
            contacts: [],
            selectedContacts: {},
          ),
        );

  final CoreClient _coreClient;

  void loadContacts() async {
    final contacts = await _coreClient.getContacts();
    emit(state.copyWith(contacts: contacts));
  }

  Future<void> addContacts(ConversationId conversationId) async {
    for (final userName in state.selectedContacts) {
      await _coreClient.addUserToConversation(conversationId, userName);
    }
    emit(state.copyWith(selectedContacts: {}));
  }

  void toggleContact(UiContact contact) {
    final selectedContacts = HashSet<String>.from(state.selectedContacts);
    if (selectedContacts.contains(contact.userName)) {
      selectedContacts.remove(contact.userName);
    } else {
      selectedContacts.add(contact.userName);
    }
    emit(state.copyWith(
      selectedContacts: selectedContacts,
    ));
  }
}
