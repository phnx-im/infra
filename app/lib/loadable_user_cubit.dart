// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:prototype/core/api/user.dart';

part 'loadable_user_cubit.freezed.dart';

/// Ternary user state: loading, loaded some or loaded none
@freezed
sealed class LoadableUser with _$LoadableUser {
  const LoadableUser._();

  /// Initial state before or in process of loading the user
  const factory LoadableUser.loading() = LoadingUser;

  /// The user has been loaded
  ///
  /// If loading was successful, the [user] will be non-null.
  const factory LoadableUser.loaded(User? user) = LoadedUser;

  User? get user => switch (this) {
        LoadingUser() => null,
        LoadedUser(:final user) => user,
      };
}

/// Observe the [User] state as [LoadableUser] initialized from a [User] stream
///
/// Can be plugged into a [BlocProvider].
class LoadableUserCubit implements StateStreamableSource<LoadableUser> {
  LoadableUserCubit(Stream<User?> stream) {
    // forward the stream to an internal broadcast stream
    _subscription = stream.listen((user) {
      _state = LoadableUser.loaded(user);
      _controller.add(_state);
    });
  }

  LoadableUser _state = const LoadableUser.loading();
  final StreamController<LoadableUser> _controller =
      StreamController.broadcast();
  late final StreamSubscription<User?> _subscription;

  @override
  FutureOr<void> close() async {
    await _subscription.cancel();
    await _controller.close();
  }

  @override
  bool get isClosed => _controller.isClosed;

  @override
  LoadableUser get state => _state;

  @override
  Stream<LoadableUser> get stream => _controller.stream;
}
