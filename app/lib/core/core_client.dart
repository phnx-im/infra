// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:logging/logging.dart';
import 'package:path_provider/path_provider.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/util/platform.dart';
import 'package:uuid/uuid_value.dart';

final _log = Logger('CoreClient');

Future<String> dbPath() async {
  final String path;

  if (Platform.isAndroid || Platform.isIOS) {
    path = await getDatabaseDirectoryMobile();
  } else {
    final directory = await getApplicationDocumentsDirectory();
    path = directory.path;
  }

  _log.info("Database path: $path");
  return path;
}

class CoreClient {
  static final CoreClient _coreClient = CoreClient._internal();

  factory CoreClient() {
    return _coreClient;
  }

  CoreClient._internal();

  User? _user;

  final StreamController<User?> _userController = StreamController<User?>();

  User? get maybeUser => _user;

  Stream<User?> get userStream => _userController.stream;

  User get user => _user!;

  set user(User? user) {
    _log.info("setting user: ${user?.userName}");
    _userController.add(user);
    _user = user;
  }

  void logout() {
    user = null;
  }

  // used in dev settings
  Future<void> deleteDatabase() async {
    await deleteDatabases(dbPath: await dbPath());
    _userController.add(null);
    _user = null;
  }

  // used in dev settings
  Future<void> deleteUserDatabase() async {
    await deleteClientDatabase(
      dbPath: await dbPath(),
      userName: user.userName,
      clientId: user.clientId,
    );
    _userController.add(null);
    _user = null;
  }

  // used in app initialization
  Future<void> loadDefaultUser() async {
    user = await User.loadDefault(path: await dbPath());
  }

  // used in registration cubit
  Future<void> createUser(
    String userName,
    String password,
    String address,
    String? displayName,
    Uint8List? profilePicture,
  ) async {
    PlatformPushToken? pushToken;

    if (Platform.isAndroid) {
      final String? deviceToken = await getDeviceToken();

      if (deviceToken != null) {
        pushToken = PlatformPushToken.google(deviceToken);
      }
    } else if (Platform.isIOS) {
      final String? deviceToken = await getDeviceToken();

      if (deviceToken != null) {
        pushToken = PlatformPushToken.apple(deviceToken);
      }
    }

    user = await User.newInstance(
      userName: userName,
      password: password,
      address: address,
      path: await dbPath(),
      pushToken: pushToken,
      displayName: displayName,
      profilePicture: profilePicture,
    );

    _log.info("User registered");
  }

  Future<void> loadUser({
    required UiUserName userName,
    required UuidValue clientId,
  }) async {
    user = await User.load(
      dbPath: await dbPath(),
      userName: userName,
      clientId: clientId,
    );
  }
}
