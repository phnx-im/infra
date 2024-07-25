import 'dart:io';
import 'package:flutter/services.dart';

const platform = MethodChannel('im.phnx.prototype/channel');

Future<String?> getDeviceToken() async {
  // Make sure we are on iOS
  if (!Platform.isIOS) {
    return null;
  }
  try {
    return await platform.invokeMethod('devicetoken');
  } on PlatformException catch (e) {
    print("Failed to get device token: '${e.message}'.");
    return null;
  }
}
