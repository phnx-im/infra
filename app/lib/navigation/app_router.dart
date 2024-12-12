// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:logging/logging.dart';
import 'package:prototype/conversation_pane/conversation_details/add_members.dart';
import 'package:prototype/conversation_pane/conversation_details/conversation_details.dart';
import 'package:prototype/conversation_pane/conversation_pane.dart';
import 'package:prototype/homescreen.dart';
import 'package:prototype/intro_screen.dart';
import 'package:prototype/registration/registration.dart';
import 'package:prototype/settings/developer.dart';
import 'package:prototype/settings/user.dart';
import 'package:prototype/theme/theme.dart';

import 'navigation_cubit.dart';

final _log = Logger('AppRouter');

class EmptyConfig {
  const EmptyConfig();
}

class AppRouter implements RouterConfig<EmptyConfig> {
  AppRouter();

  final AppRouterDelegate _routerDelegate = AppRouterDelegate();

  final AppBackButtonDispatcher _backButtonDispatcher =
      AppBackButtonDispatcher();

  @override
  BackButtonDispatcher? get backButtonDispatcher => _backButtonDispatcher;

  @override
  RouteInformationParser<EmptyConfig>? get routeInformationParser => null;

  @override
  RouteInformationProvider? get routeInformationProvider => null;

  @override
  RouterDelegate<EmptyConfig> get routerDelegate => _routerDelegate;
}

/// The main application router
///
/// Builds pages from the navigation state [NavigationState] provided by the
/// [NavigationCubit]. This is where the translation from the navigation
/// state to the actual list of pages happens.
class AppRouterDelegate extends RouterDelegate<EmptyConfig> {
  AppRouterDelegate();

  final GlobalKey<NavigatorState> _navigatorKey = GlobalKey<NavigatorState>();

  final PageStorageBucket _bucket = PageStorageBucket();

  @override
  Widget build(BuildContext context) {
    final navigationState = context.watch<NavigationCubit>().state;

    // hide material banners if any
    ScaffoldMessenger.of(context).hideCurrentMaterialBanner();

    final screenType = context.responsiveScreenType;

    // routing
    final List<MaterialPage> pages = switch (navigationState) {
      IntroNavigation(screens: final screensStack) => [
          for (final screenType in screensStack)
            MaterialPage(key: screenType.key, child: screenType.screen),
        ],
      HomeNavigation home => home.pages(screenType),
    };

    _log.finer(
        "AppRouterDelegate.build: navigationState = $navigationState, pages=$pages");

    return PageStorage(
      bucket: _bucket,
      child: Navigator(
        key: _navigatorKey,
        pages: pages,
        // Note: onPopPage is deprecated, and instead we should use
        // onDidRemovePage. However, the latter does not allow to distinguish
        // whether the page was popped by the user or programmatically.
        //
        // Also see
        //   * <https://github.com/phnx-im/infra/issues/244>
        //   * <https://github.com/flutter/flutter/issues/109494>
        //
        // ignore: deprecated_member_use
        onPopPage: (route, result) {
          // check whether the page was popped by the back button
          if (!route.didPop(result)) {
            return false;
          }
          if (route.settings case MaterialPage _) {
            return context.read<NavigationCubit>().pop();
          }
          return false;
        },
      ),
    );
  }

  /// Back button handler
  @override
  Future<bool> popRoute() {
    return SynchronousFuture(
      _navigatorKey.currentContext?.read<NavigationCubit>().pop() ?? false,
    );
  }

  @override
  void addListener(VoidCallback listener) {
    // Listening to the navigation state is not supported.
  }

  @override
  void removeListener(VoidCallback listener) {
    // Listening to the navigation state is not supported.
  }

  @override
  Future<void> setNewRoutePath(EmptyConfig configuration) async {
    // This called in Web when an URL is entered in the browser, or when `Router.navigate` is called
    // programmatically. We dont handle these cases.
  }
}

class AppBackButtonDispatcher extends RootBackButtonDispatcher {}

/// Convert an [IntroScreenType] into a [ValueKey] and a screen [Widget].
extension on IntroScreenType {
  ValueKey<String> get key => switch (this) {
        IntroScreenType.intro => const ValueKey("intro-screen"),
        IntroScreenType.serverChoice => const ValueKey("server-choice-screen"),
        IntroScreenType.usernamePassword =>
          const ValueKey("username-password-screen"),
        IntroScreenType.displayNamePicture =>
          const ValueKey("display-name-picture-screen"),
        IntroScreenType.developerSettings =>
          const ValueKey("developer-settings-screen"),
      };

  Widget get screen => switch (this) {
        IntroScreenType.intro => const IntroScreen(),
        IntroScreenType.serverChoice => const ServerChoice(),
        IntroScreenType.usernamePassword => const UsernamePasswordChoice(),
        IntroScreenType.displayNamePicture => const DisplayNameAvatarChoice(),
        IntroScreenType.developerSettings => const DeveloperSettingsScreen(),
      };
}

/// Convert [HomeNavigation] state into a list of pages.
extension on HomeNavigation {
  List<MaterialPage> pages(ResponsiveScreenType screenType) {
    const homeScreenPage = MaterialPage(
      key: ValueKey("home-screen"),
      child: HomeScreen(),
    );
    return [
      homeScreenPage,
      if (userSettingsOpen)
        const MaterialPage(
          key: ValueKey("user-settings-screen"),
          child: UserSettingsScreen(),
        ),
      if (conversationId != null && screenType == ResponsiveScreenType.mobile)
        const MaterialPage(
          key: ValueKey("conversation-screen"),
          child: ConversationPane(),
        ),
      if (conversationId != null && conversationDetailsOpen)
        const MaterialPage(
          key: ValueKey("conversation-details-screen"),
          child: ConversationDetails(),
        ),
      if (conversationId != null && conversationDetailsOpen && addMembersOpen)
        const MaterialPage(
          key: ValueKey("add-members-screen"),
          child: AddMembers(),
        ),
      if (developerSettingsOpen)
        const MaterialPage(
          key: ValueKey("developer-settings-screen"),
          child: DeveloperSettingsScreen(),
        ),
    ];
  }
}
