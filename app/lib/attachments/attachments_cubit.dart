// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/user/user.dart';

/// Repository of all attachments
class AttachmentsCubit implements StateStreamableSource<AttachmentsState> {
  AttachmentsCubit({required UserCubit userCubit})
    : _impl = AttachmentsCubitBase(userCubit: userCubit.impl);

  final AttachmentsCubitBase _impl;

  @override
  FutureOr<void> close() => _impl.close();

  @override
  bool get isClosed => _impl.isClosed;

  @override
  AttachmentsState get state => _impl.state;

  @override
  Stream<AttachmentsState> get stream => _impl.stream();
}
