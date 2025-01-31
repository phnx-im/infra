// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

class ConversationListHeader extends StatelessWidget {
  const ConversationListHeader({
    super.key,
  });

  double _topOffset() {
    return isPointer() ? 30 : kToolbarHeight;
  }

  double _topHeight() {
    return 60 + _topOffset();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        SizedBox(
          height: _topHeight(),
          child: FrostedGlass(
            color: convPaneBackgroundColor,
            height: _topHeight(),
          ),
        ),
        Padding(
          padding: EdgeInsets.only(left: 8, right: 8, top: _topOffset()),
          child: const Row(
            children: [
              _Avatar(),
              Expanded(
                child: _UsernameSpace(),
              ),
              _SettingsButton(),
            ],
          ),
        ),
      ],
    );
  }
}

class _Avatar extends StatelessWidget {
  const _Avatar();

  @override
  Widget build(BuildContext context) {
    final (userName, profilePicture) = context.select(
      (UserCubit cubit) => (
        cubit.state.userName,
        cubit.state.profilePicture,
      ),
    );

    return Padding(
      padding: const EdgeInsets.only(left: 18.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          UserAvatar(
            size: 32,
            username: userName,
            image: profilePicture,
            onPressed: () {
              context.read<NavigationCubit>().openUserSettings();
            },
          )
        ],
      ),
    );
  }
}

class _UsernameSpace extends StatelessWidget {
  const _UsernameSpace();

  @override
  Widget build(BuildContext context) {
    final (userName, displayName) = context.select(
      (UserCubit cubit) => (
        cubit.state.userName,
        cubit.state.displayName,
      ),
    );

    return Column(
      children: [
        Text(
          displayName ?? "",
          style: const TextStyle(
            color: colorDMB,
            fontSize: 13,
          ).merge(VariableFontWeight.bold),
        ),
        const SizedBox(height: 5),
        Text(
          userName,
          style: const TextStyle(
            color: colorDMB,
            fontSize: 10,
          ).merge(VariableFontWeight.medium),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }
}

class _SettingsButton extends StatelessWidget {
  const _SettingsButton();

  @override
  Widget build(BuildContext context) {
    return IconButton(
      onPressed: () {
        context.read<NavigationCubit>().openDeveloperSettings();
      },
      hoverColor: Colors.transparent,
      focusColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      icon: const Icon(
        Icons.settings,
        size: 20,
        color: colorDMB,
      ),
    );
  }
}
