// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/user/user.dart';

/// Repository of all user profiles including the logged-in user.
class ContactsCubit implements StateStreamableSource<ContactsState> {
  ContactsCubit({required UserCubit userCubit})
    : _impl = ContactsCubitBase(userCubit: userCubit.impl);

  final ContactsCubitBase _impl;

  @override
  FutureOr<void> close() => _impl.close();

  @override
  bool get isClosed => _impl.isClosed;

  @override
  ContactsState get state => _impl.state;

  @override
  Stream<ContactsState> get stream => _impl.stream();
}
