// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';

part 'navigation_cubit.freezed.dart';

/// State of the global app navigation
///
/// Accessible via [NavigationCubit.state] from anywhere within the app.
@freezed
sealed class NavigationState with _$NavigationState {
  const NavigationState._();

  /// Intro screen: welcome and login screen
  const factory NavigationState.intro({
    @Default([IntroScreenType.intro]) List<IntroScreenType> screens,
  }) = IntroNavigation;

  /// Conversations screen: main screen of the app
  ///
  /// Note: this can be represented in a better way disallowing invalid states.
  /// For now, following KISS we represent the navigation stack in a very simple
  /// way by just storing true/false or an optional value representing if a
  /// screen is opened.
  const factory NavigationState.home({
    ConversationId? conversationId,
    DeveloperSettingsScreenType? developerSettingsScreen,
    @Default(false) bool userSettingsOpen,
    @Default(false) bool conversationDetailsOpen,
    @Default(false) bool addMembersOpen,

    /// User name of the member that details are currently open
    String? memberDetails,
  }) = HomeNavigation;

  ConversationId? get conversationId => switch (this) {
        IntroNavigation() => null,
        HomeNavigation(:final conversationId) => conversationId,
      };
}

/// Possible intro screens
enum IntroScreenType {
  intro,
  serverChoice,
  usernamePassword,
  displayNamePicture,
  developerSettings,
}

enum DeveloperSettingsScreenType { root, changeUser, logs }

/// Provides the navigation state and navigation actions to the app
///
/// This is main entry point for navigation.
///
/// For the actual translation of the state to the actual screens, see
/// `AppRouter`.
class NavigationCubit extends Cubit<NavigationState> {
  NavigationCubit() : super(const NavigationState.intro());

  void openIntro() {
    emit(const NavigationState.intro());
  }

  void openHome() {
    emit(const NavigationState.home());
  }

  void openConversation(ConversationId conversationId) {
    switch (state) {
      case IntroNavigation():
        emit(const NavigationState.home());
        return;
      case HomeNavigation homeState:
        emit(homeState.copyWith(conversationId: conversationId));
    }
  }

  bool closeConversation() {
    if (state case HomeNavigation homeState
        when homeState.conversationId != null) {
      emit(homeState.copyWith(conversationId: null));
      return true;
    } else {
      return false;
    }
  }

  void openConversationDetails() {
    if (state case HomeNavigation homeState) {
      if (homeState.conversationId != null) {
        emit(homeState.copyWith(conversationDetailsOpen: true));
      }
      return;
    }
    throw NavigationError(state);
  }

  void openMemberDetails(String member) {
    if (state case HomeNavigation homeState) {
      emit(homeState.copyWith(memberDetails: member));
      return;
    }
    throw NavigationError(state);
  }

  void openAddMembers() {
    if (state case HomeNavigation homeState
        when homeState.conversationDetailsOpen) {
      emit(homeState.copyWith(addMembersOpen: true));
      return;
    }
    throw NavigationError(state);
  }

  void openUserSettings() {
    if (state case HomeNavigation homeState) {
      emit(homeState.copyWith(userSettingsOpen: true));
      return;
    }
    throw NavigationError(state);
  }

  void openDeveloperSettings({
    DeveloperSettingsScreenType screen = DeveloperSettingsScreenType.root,
  }) {
    switch (state) {
      case IntroNavigation intro:
        if (intro.screens.lastOrNull != IntroScreenType.developerSettings) {
          final stack = [...intro.screens, IntroScreenType.developerSettings];
          emit(intro.copyWith(screens: stack));
        }
      case HomeNavigation home:
        emit(home.copyWith(developerSettingsScreen: screen));
    }
  }

  void openServerChoice() {
    if (state case IntroNavigation intro) {
      if (intro.screens.lastOrNull != IntroScreenType.serverChoice) {
        emit(intro.copyWith(
            screens: [...intro.screens, IntroScreenType.serverChoice]));
      }
      return;
    }
    throw NavigationError(state);
  }

  void openIntroScreen(IntroScreenType screen) {
    if (state case IntroNavigation intro) {
      if (intro.screens.lastOrNull != screen) {
        emit(intro.copyWith(screens: [...intro.screens, screen]));
      }
      return;
    }
    throw NavigationError(state);
  }

  void openDisplayNamePicture() {
    if (state case IntroNavigation intro) {
      if (intro.screens.lastOrNull != IntroScreenType.displayNamePicture) {
        emit(intro.copyWith(
            screens: [...intro.screens, IntroScreenType.displayNamePicture]));
      }
      return;
    }
    throw NavigationError(state);
  }

  bool pop() {
    switch (state) {
      case IntroNavigation(screens: final screens):
        if (screens.length > 1) {
          emit(
            IntroNavigation(screens: screens.sublist(0, screens.length - 1)),
          );
          return true;
        }
        return false;
      case HomeNavigation home:
        if (home.developerSettingsScreen != null) {
          switch (home.developerSettingsScreen) {
            case null:
              throw StateError("impossible state");
            case DeveloperSettingsScreenType.root:
              emit(home.copyWith(developerSettingsScreen: null));
              return true;
            case DeveloperSettingsScreenType.changeUser ||
                  DeveloperSettingsScreenType.logs:
              emit(home.copyWith(
                developerSettingsScreen: DeveloperSettingsScreenType.root,
              ));
              return true;
          }
        } else if (home.userSettingsOpen) {
          emit(home.copyWith(userSettingsOpen: false));
          return true;
        } else if (home.memberDetails != null) {
          emit(home.copyWith(memberDetails: null));
          return true;
        } else if (home.conversationId != null && home.addMembersOpen) {
          emit(home.copyWith(addMembersOpen: false));
          return true;
        } else if (home.conversationId != null &&
            home.conversationDetailsOpen) {
          emit(home.copyWith(conversationDetailsOpen: false));
          return true;
        } else if (home.conversationId != null) {
          emit(const HomeNavigation());
          return true;
        }
        return false;
    }
  }
}

final class NavigationError extends StateError {
  NavigationError(NavigationState state)
      : super("Failed to open screen: $state");
}
