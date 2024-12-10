// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:logging/logging.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/platform.dart';
import 'package:prototype/observable_user.dart';
import 'package:provider/provider.dart';

import 'registration/registration.dart';
import 'theme/theme.dart';

final _log = Logger('App');

final _appRouter = AppRouter();

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
    return MaterialApp.router(
      title: 'Prototype',
      debugShowCheckedModeBanner: false,
      theme: themeData(context),
      routerConfig: _appRouter,
      builder: (context, router) => MultiBlocProvider(
        providers: [
          Provider.value(value: _coreClient),
          BlocProvider<NavigationCubit>(create: (context) => NavigationCubit()),
          BlocProvider<RegistrationCubit>(
              create: (context) => RegistrationCubit(coreClient: _coreClient)),
          BlocProvider<ObservableUser>(
            create: (context) =>
                // loads the user on startup
                ObservableUser((_coreClient..loadUser()).userStream),
          ),
        ],
        child: BlocListener<ObservableUser, UserState>(
          listenWhen: (previous, current) =>
              // only fire the side effect when the user logs in or out
              (current.user == null || previous.user == null) &&
              current.user != previous.user,
          listener: (context, user) {
            // Side Effect: navigate to the home screen or away to the intro
            // screen, depending on whether the user was loaded or unloaded.
            switch (user) {
              case LoadedUserState(user: final _?):
                context.read<NavigationCubit>().openHome();
              case LoadingUserState() || LoadedUserState(user: null):
                context.read<NavigationCubit>().openIntro();
            }
          },
          child: router!,
        ),
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
