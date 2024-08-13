// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';
import 'package:flutter/services.dart';

const platform = MethodChannel('im.phnx.prototype/channel');

void initMethodChannel() {
  platform.setMethodCallHandler(_handleMethod);
}

Future<void> _handleMethod(MethodCall call) async {
  switch (call.method) {
    case 'receivedNotification':
      // Handle notification data
      final String data = call.arguments["customData"];
      print('Notification data: $data');
      // Do something with the data
      break;
    case 'openedNotification':
      // Handle notification opened
      final String data = call.arguments["customData"];
      print('Notification opened: $data');
      // Do something with the data
      break;
    default:
      print('Unknown method called: ${call.method}');
  }
}

Future<String?> getDeviceToken() async {
  // Make sure we are on iOS
  if (!Platform.isIOS) {
    return null;
  }
  try {
    return await platform.invokeMethod('getDeviceToken');
  } on PlatformException catch (e) {
    print("Failed to get device token: '${e.message}'.");
    return null;
  }
}

Future<String> getSharedDocumentsDirectoryIos() async {
  // Make sure we are on iOS
  if (!Platform.isIOS) {
    throw PlatformException(code: 'platform_not_supported');
  }
  try {
    return await platform.invokeMethod('getSharedDocumentsDirectory');
  } on PlatformException catch (e) {
    print("Failed to get shared documents directory: '${e.message}'.");
    throw PlatformException(code: 'failed_to_get_shared_documents_directory');
  }
}

Future<void> setBadgeCount(int count) async {
  // Make sure we are on iOS
  if (!Platform.isIOS) {
    return;
  }
  try {
    await platform.invokeMethod('setBadgeCount', count);
  } on PlatformException catch (e) {
    print("Failed to set badge count: '${e.message}'.");
  }
}
