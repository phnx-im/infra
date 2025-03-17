// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:async';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';

export 'package:prototype/core/core.dart'
    show NavigationState, IntroScreenType, DeveloperSettingsScreenType;
export 'package:prototype/core/core_extension.dart'
    show NavigationStateExtension;

class NavigationCubit implements StateStreamableSource<NavigationState> {
  NavigationCubit() : _impl = NavigationCubitBase();

  final NavigationCubitBase _impl;

  NavigationCubitBase get base => _impl;

  @override
  FutureOr<void> close() => _impl.close();

  @override
  bool get isClosed => _impl.isClosed;

  @override
  NavigationState get state => _impl.state;

  @override
  Stream<NavigationState> get stream => _impl.stream();

  // Methods

  Future<void> closeConversation() => _impl.closeConversation();

  Future<void> openConversation(ConversationId conversationId) =>
      _impl.openConversation(conversationId: conversationId);

  Future<void> openConversationWithClearedNotifications(
          ConversationId conversationId) =>
      _impl.openConversationWithClearedNotifications(
          conversationId: conversationId);

  Future<void> openConversationDetails() => _impl.openConversationDetails();

  Future<void> openAddMembers() => _impl.openAddMembers();

  Future<void> openMemberDetails(String member) =>
      _impl.openMemberDetails(member: member);

  Future<void> openDeveloperSettings({
    DeveloperSettingsScreenType screen = DeveloperSettingsScreenType.root,
  }) =>
      _impl.openDeveloperSettings(screen: screen);

  Future<void> openHome() => _impl.openHome();

  Future<void> openIntro() => _impl.openInto();

  Future<void> openIntroScreen(IntroScreenType screen) =>
      _impl.openIntroScreen(screen: screen);

  Future<void> openUserSettings() => _impl.openUserSettings();

  bool pop() => _impl.pop();

  Future<void> openServerChoice() =>
      _impl.openIntroScreen(screen: IntroScreenType.serverChoice);
}
