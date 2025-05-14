// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:logging/logging.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/util/platform.dart';
import 'package:prototype/user/user.dart';
import 'package:provider/provider.dart';

import 'conversation_details/conversation_details.dart';
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

  final StreamController<ConversationId> _openedNotificationController =
      StreamController<ConversationId>();
  late final StreamSubscription<ConversationId> _openedNotificationSubscription;
  final NavigationCubit _navigationCubit = NavigationCubit();

  final StreamController<AppState> _appStateController =
      StreamController<AppState>.broadcast();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    initMethodChannel(_openedNotificationController.sink);
    _openedNotificationSubscription = _openedNotificationController.stream
        .listen((conversationId) {
          _navigationCubit.openConversation(conversationId);
        });

    _requestMobileNotifications();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _openedNotificationSubscription.cancel();
    _openedNotificationController.close();
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
      _appStateController.sink.add(AppState.background);

      // iOS only
      if (Platform.isIOS) {
        // only set the badge count if the user is logged in
        if (_coreClient.maybeUser case final user?) {
          final count = await user.globalUnreadMessagesCount;
          await setBadgeCount(count);
        }
      }
    } else if (state == AppLifecycleState.resumed) {
      _log.fine('App is in the foreground');
      _appStateController.sink.add(AppState.foreground);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MultiBlocProvider(
      providers: [
        Provider.value(value: _coreClient),
        BlocProvider<NavigationCubit>.value(value: _navigationCubit),
        BlocProvider<RegistrationCubit>(
          create: (context) => RegistrationCubit(coreClient: _coreClient),
        ),
        BlocProvider<LoadableUserCubit>(
          // loads the user on startup
          create:
              (context) => LoadableUserCubit(
                (_coreClient..loadDefaultUser()).userStream,
              ),
          lazy: false, // immediately try to load the user
        ),
      ],
      child: MaterialApp.router(
        title: 'Prototype',
        debugShowCheckedModeBanner: false,
        theme: themeData(context),
        routerConfig: _appRouter,
        builder:
            (context, router) => LoadableUserCubitProvider(
              appStateController: _appStateController,
              child: ConversationDetailsCubitProvider(child: router!),
            ),
      ),
    );
  }
}

class LoadableUserCubitProvider extends StatelessWidget {
  const LoadableUserCubitProvider({
    required this.appStateController,
    required this.child,
    super.key,
  });

  final StreamController<AppState> appStateController;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    // This bloc has two tasks:
    // 1. Listen to the loadable user and switch the navigation accordingly.
    // 2. Provide the logged in user to the app, when it is loaded.
    return BlocConsumer<LoadableUserCubit, LoadableUser>(
      listenWhen: _isUserLoadedOrUnloaded,
      buildWhen: _isUserLoadedOrUnloaded,
      listener: (context, loadableUser) {
        // Side Effect: navigate to the home screen or away to the intro
        // screen, depending on whether the user was loaded or unloaded.
        switch (loadableUser) {
          case LoadedUser(user: final _?):
            context.read<NavigationCubit>().openHome();
          case LoadingUser() || LoadedUser(user: null):
            context.read<NavigationCubit>().openIntro();
        }
      },
      builder:
          (context, loadableUser) =>
              loadableUser.user != null
                  // Logged-in user is accessible everywhere inside the app after
                  // the user is loaded
                  ? BlocProvider<UserCubit>(
                    create:
                        (context) => UserCubit(
                          coreClient: context.read<CoreClient>(),
                          navigationCubit: context.read<NavigationCubit>(),
                          appStateStream: appStateController.stream,
                        ),
                    child: child,
                  )
                  : child,
    );
  }
}

/// Checks if [LoadableUser.user] transitioned from loaded to null or vice versa
bool _isUserLoadedOrUnloaded(LoadableUser previous, LoadableUser current) =>
    (previous.user != null || current.user != null) &&
    previous.user != current.user;

void _requestMobileNotifications() async {
  // Mobile initialization
  if (Platform.isAndroid || Platform.isIOS) {
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

/// Creates a [ConversationDetailsCubit] for the current conversation
///
/// This is used to mount the conversation details cubit when the user
/// navigates to a conversation. The [ConversationDetailsCubit] can be
/// then used from any screen.
class ConversationDetailsCubitProvider extends StatelessWidget {
  const ConversationDetailsCubitProvider({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<NavigationCubit, NavigationState>(
      buildWhen:
          (previous, current) =>
              current.conversationId != previous.conversationId,
      builder: (context, state) {
        final conversationId = state.conversationId;
        if (conversationId == null) {
          return child;
        }
        return BlocProvider(
          create:
              (context) => ConversationDetailsCubit(
                userCubit: context.read<UserCubit>(),
                conversationId: conversationId,
              ),
          child: child,
        );
      },
    );
  }
}
