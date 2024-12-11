// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core_client.dart';

part 'registration_cubit.freezed.dart';

final _log = Logger('RegistrationCubit');

// * It consists of one or more labels separated by dots.
// * Each label can contain alphanumeric characters (A-Z, a-z, 0-9) and hyphens.
// * Labels cannot start or end with a hyphen.
// * Each label must be between 1 and 63 characters long.
final _domainRegex =
    RegExp(r'^(?!-)[A-Za-z0-9-]{1,63}(?<!-)(\.[A-Za-z0-9-]{1,63})*$');
final _usernameRegex = RegExp(r'^[a-zA-Z0-9@.]+$');

@freezed
sealed class RegistrationState with _$RegistrationState {
  const RegistrationState._();

  const factory RegistrationState({
    // Domain choice screen data
    @Default('') String domain,

    // Username/password screen data
    @Default('') String username,
    @Default('') String password,
    @Default(false) bool isUsernameValid,
    @Default(false) bool isPasswordValid,

    // Display name/avatar screen data
    Uint8List? avatar,
    String? displayName,
    @Default(false) bool isSigningUp,
  }) = _RegistrationState;

  get isDomainValid => _domainRegex.hasMatch(domain);
}

class RegistrationCubit extends Cubit<RegistrationState> {
  RegistrationCubit({
    required CoreClient coreClient,
  })  : _coreClient = coreClient,
        super(const RegistrationState());

  final CoreClient _coreClient;

  void setDomain(String value) {
    emit(state.copyWith(domain: value));
  }

  void setUsername(String value) {
    var containsInvalidChars =
        value.isNotEmpty && !_usernameRegex.hasMatch(value);
    var hasRightLength = value.isNotEmpty && value.length <= 64;
    emit(state.copyWith(
      isUsernameValid: hasRightLength && !containsInvalidChars,
      username: value,
      displayName: value,
    ));
  }

  void setPassword(String value) {
    emit(state.copyWith(
      password: value,
      isPasswordValid: value.isNotEmpty,
    ));
  }

  void setAvatar(Uint8List? bytes) {
    emit(state.copyWith(avatar: bytes));
  }

  void setDisplayName(String value) {
    emit(state.copyWith(displayName: value));
  }

  Future<SignUpError?> signUp() async {
    emit(state.copyWith(isSigningUp: true));

    final fqun = "${state.username}@${state.domain}";
    final url = "https://${state.domain}";

    try {
      _log.info("Registering user ${state.username} ...");
      await _coreClient.createUser(fqun, state.password, url);
    } catch (e) {
      final message = "Error when registering user: ${e.toString()}";
      _log.severe(message);
      emit(state.copyWith(isSigningUp: false));
      return SignUpError(message);
    }

    // Set the user's display name and profile picture
    try {
      await _coreClient.setOwnProfile(state.displayName ?? "", state.avatar);
    } catch (e) {
      final message = "Error when setting profile: $e";
      _log.severe(message);
      emit(state.copyWith(isSigningUp: false));
      return SignUpError(message);
    }

    return null;
  }
}

final class SignUpError {
  const SignUpError(this.message);
  final String message;
}
