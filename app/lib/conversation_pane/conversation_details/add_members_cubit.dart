// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:prototype/core/api/types.dart';

part 'add_members_cubit.freezed.dart';

@freezed
class AddMembersState with _$AddMembersState {
  const factory AddMembersState({
    required List<UiContact> contacts,
    required Set<String> selectedContacts,
  }) = _AddMembersState;
}

class AddMembersCubit extends Cubit<AddMembersState> {
  AddMembersCubit()
      : super(
          const AddMembersState(
            contacts: [],
            selectedContacts: {},
          ),
        );

  void loadContacts(Future<List<UiContact>> contacts) async {
    emit(state.copyWith(contacts: await contacts));
  }

  void toggleContact(UiContact contact) {
    final selectedContacts = Set<String>.from(state.selectedContacts);
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
