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

  static height(BuildContext context) =>
      MediaQuery.of(context).padding.top + kToolbarHeight;

  @override
  Widget build(BuildContext context) {
    final topPadding = MediaQuery.of(context).padding.top;
    const height = kToolbarHeight;

    return Stack(
      children: [
        SizedBox(
          height: topPadding + height,
          child: FrostedGlass(
            color: convPaneBackgroundColor,
            height: topPadding + height,
          ),
        ),
        Container(
          height: topPadding + height,
          padding: EdgeInsets.only(top: topPadding),
          child: const Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              _Avatar(),
              Spacer(),
              _UsernameSpace(),
              Spacer(),
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
      mainAxisAlignment: MainAxisAlignment.center,
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
