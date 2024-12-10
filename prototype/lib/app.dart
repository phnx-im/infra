// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/homescreen.dart';
import 'package:prototype/platform.dart';
import 'package:provider/provider.dart';

import 'theme/theme.dart';

final GlobalKey<NavigatorState> appNavigator = GlobalKey<NavigatorState>();

final _log = Logger('App');

class App extends StatefulWidget {
  const App({super.key});

  @override
  State<App> createState() => _AppState();
}

class _AppState extends State<App> with WidgetsBindingObserver {
  final CoreClient _coreClient = CoreClient();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _requestMobileNotifications();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    super.didChangeAppLifecycleState(state);
    _onStateChanged(state);
  }

  Future<void> _onStateChanged(AppLifecycleState state) async {
    if (state == AppLifecycleState.paused) {
      _log.fine('App is in the background');

      // iOS only
      if (Platform.isIOS) {
        // only set the badge count if the user is logged in
        if (_coreClient.maybeUser case final user?) {
          final count = await user.globalUnreadMessagesCount();
          await setBadgeCount(count);
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    // TODO: This provider should be moved below the `MaterialApp`. This can be
    // done when the app router is introduced. We can't just wrap the
    // `HomeScreen` because it is replaced in other places by another screens.
    return Provider.value(
      value: _coreClient,
      child: MaterialApp(
        title: 'Prototype',
        debugShowCheckedModeBanner: false,
        theme: themeData(context),
        navigatorKey: appNavigator,
        home: const HomeScreen(),
      ),
    );
  }
}

void _requestMobileNotifications() async {
  // Mobile initialization
  if (Platform.isAndroid || Platform.isIOS) {
    // Initialize the method channel
    initMethodChannel();

    // Ask for notification permission
    var status = await Permission.notification.status;
    switch (status) {
      case PermissionStatus.denied:
        _log.info("Notification permission denied, will ask the user");
        var requestStatus = await Permission.notification.request();
        _log.fine("The status is $requestStatus");
        break;
      default:
        _log.info("Notification permission status: $status");
    }
  }
}
