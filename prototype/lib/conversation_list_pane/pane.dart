// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/conversation_list.dart';
import 'package:prototype/conversation_list_pane/footer.dart';
import 'package:prototype/conversation_list_pane/top.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/styles.dart';

class ConversationView extends StatefulWidget {
  const ConversationView({super.key});

  @override
  State<ConversationView> createState() => _ConversationViewState();
}

class _ConversationViewState extends State<ConversationView> {
  String? displayName = coreClient.ownProfile.displayName;
  Uint8List? profilePicture = coreClient.ownProfile.profilePictureOption;

  @override
  void initState() {
    super.initState();
    // Listen for changes to the user's profile picture
    coreClient.onOwnProfileUpdate.listen((profile) {
      if (mounted) {
        setState(() {
          profilePicture = profile.profilePictureOption;
          displayName = profile.displayName;
        });
      }
    });
  }

  @override
  Widget build(
    BuildContext context,
  ) {
    return Container(
      decoration: const BoxDecoration(
        shape: BoxShape.rectangle,
        border: Border(
          right: BorderSide(
            width: 1,
            color: colorGreyLight,
          ),
        ),
      ),
      child: Scaffold(
        backgroundColor: convPaneBackgroundColor,
        body: Stack(
          children: [
            const ConversationList(),
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: ConversationListTop(
                  displayName: displayName, profilePicture: profilePicture),
            ),
            const Positioned(
              bottom: 0,
              left: 0,
              right: 0,
              child: ConversationListFooter(),
            ),
          ],
        ),
      ),
    );
  }
}
