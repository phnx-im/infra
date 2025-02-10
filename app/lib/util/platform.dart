// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';
import 'package:flutter/services.dart';
import 'package:logging/logging.dart';

const platform = MethodChannel('im.phnx.prototype/channel');

final _log = Logger('Platform');

void initMethodChannel() {
  platform.setMethodCallHandler(_handleMethod);
}

Future<void> _handleMethod(MethodCall call) async {
  switch (call.method) {
    case 'receivedNotification':
      // Handle notification data
      final String data = call.arguments["customData"];
      _log.info('Notification data: $data');
      // Do something with the data
      break;
    case 'openedNotification':
      // Handle notification opened
      final String data = call.arguments["customData"];
      _log.info('Notification opened: $data');
      // Do something with the data
      break;
    default:
      _log.severe('Unknown method called: ${call.method}');
  }
}

Future<String?> getDeviceToken() async {
  if (Platform.isAndroid || Platform.isIOS) {
    try {
      return await platform.invokeMethod('getDeviceToken');
    } on PlatformException catch (e, stacktrace) {
      _log.severe("Failed to get device token: '${e.message}'.", e, stacktrace);
    }
  }
  return null;
}

Future<String> getDatabaseDirectoryMobile() async {
  if (Platform.isAndroid || Platform.isIOS) {
    try {
      return await platform.invokeMethod('getDatabasesDirectory');
    } on PlatformException catch (e, stacktrace) {
      _log.severe(
          "Failed to get database directory: '${e.message}'.", e, stacktrace);
      throw PlatformException(code: 'failed_to_get_database_directory');
    }
  }
  return '';
}

Future<void> setBadgeCount(int count) async {
  // Make sure we are on iOS
  if (!Platform.isIOS) {
    return;
  }
  try {
    await platform.invokeMethod('setBadgeCount', {'count': count});
  } on PlatformException catch (e, stacktrace) {
    _log.severe("Failed to set badge count: '${e.message}'.", e, stacktrace);
  }
}
