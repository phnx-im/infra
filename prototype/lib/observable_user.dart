// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:prototype/core/api/user.dart';

part 'observable_user.freezed.dart';

/// Ternary user state: loading, loaded some or loaded none
@freezed
sealed class UserState with _$UserState {
  const UserState._();

  /// Initial state before or in process of loading the user
  const factory UserState.loading() = LoadingUserState;

  /// The user has been loaded
  ///
  /// If loading was successful, the [user] will be non-null.
  const factory UserState.loaded(User? user) = LoadedUserState;

  User? get user => switch (this) {
        LoadingUserState() => null,
        LoadedUserState(:final user) => user,
      };
}

/// Observe the [User] state as [UserState] initialized from a [User] stream
///
/// Can be plugged into a [BlocProvider].
class ObservableUser implements StateStreamableSource<UserState> {
  ObservableUser(Stream<User?> stream) {
    // forward the stream to an internal broadcast stream
    _subscription = stream.listen((user) {
      _state = UserState.loaded(user);
      _controller.add(_state);
    });
  }

  UserState _state = const UserState.loading();
  final StreamController<UserState> _controller = StreamController.broadcast();
  late final StreamSubscription<User?> _subscription;

  @override
  FutureOr<void> close() async {
    await _subscription.cancel();
    await _controller.close();
  }

  @override
  bool get isClosed => _controller.isClosed;

  @override
  UserState get state => _state;

  @override
  Stream<UserState> get stream => _controller.stream;
}
