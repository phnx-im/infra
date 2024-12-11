// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';

class ConversationListTop extends StatelessWidget {
  const ConversationListTop({
    super.key,
    required this.displayName,
    required this.profilePicture,
  });

  final String? displayName;
  final Uint8List? profilePicture;

  double _topOffset() {
    return isPointer() ? 30 : kToolbarHeight;
  }

  double _topHeight() {
    return 60 + _topOffset();
  }

  Widget _avatar(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 18.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          UserAvatar(
            size: 32,
            username: context.coreClient.username,
            image: profilePicture,
            onPressed: () {
              context.read<NavigationCubit>().openUserSettings();
            },
          )
        ],
      ),
    );
  }

  Column _usernameSpace(String username) {
    return Column(
      children: [
        Text(
          displayName ?? "",
          style: const TextStyle(
            color: colorDMB,
            fontVariations: variationBold,
            fontSize: 13,
            letterSpacing: -0.2,
          ),
        ),
        const SizedBox(height: 5),
        Text(
          username,
          style: const TextStyle(
            color: colorDMB,
            fontSize: 10,
            fontVariations: variationMedium,
            letterSpacing: -0.2,
          ),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }

  Widget _settingsButton(BuildContext context) {
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

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        SizedBox(
          height: _topHeight(),
          child: FrostedGlass(
              color: convPaneBackgroundColor, height: _topHeight()),
        ),
        Padding(
          padding: EdgeInsets.only(left: 8, right: 8, top: _topOffset()),
          child: Row(
            children: [
              _avatar(context),
              Expanded(
                child: _usernameSpace(context.coreClient.username),
              ),
              _settingsButton(context),
            ],
          ),
        ),
      ],
    );
  }
}
