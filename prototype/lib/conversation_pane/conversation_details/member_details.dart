// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:prototype/core/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';

// Constant for padding between the elements
const double _padding = 32;

class MemberDetails extends StatefulWidget {
  final UiConversation conversation;
  final String username;

  const MemberDetails(
      {super.key, required this.conversation, required this.username});

  @override
  State<MemberDetails> createState() => _MemberDetailsState();
}

class _MemberDetailsState extends State<MemberDetails> {
  late StreamSubscription<UiConversation> _conversationListener;

  @override
  void initState() {
    super.initState();
    // Listen for conversation switch events and close the member details pane
    // when the conversation changes
    _conversationListener = coreClient.onConversationSwitch.listen((event) {
      Navigator.of(context).pop();
    });
  }

  @override
  void dispose() {
    _conversationListener.cancel();
    super.dispose();
  }

  bool isSelf() {
    return widget.username == coreClient.username;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: appBarBackButton(context),
        title: const Text("Member details"),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Column(
              mainAxisAlignment: MainAxisAlignment.start,
              children: [
                const SizedBox(height: _padding),
                FutureUserAvatar(
                  size: 64,
                  profile:
                      coreClient.user.userProfile(userName: widget.username),
                ),
                const SizedBox(height: _padding),
                Text(
                  widget.username,
                  style: labelStyle,
                ),
                const SizedBox(height: _padding),
              ],
            ),
            // Show the remove user button if the user is not the current user
            (!isSelf())
                ? Padding(
                    padding: const EdgeInsets.all(_padding),
                    child: OutlinedButton(
                        onPressed: () async {
                          bool confirmed = await showDialog(
                            context: context,
                            builder: (BuildContext context) {
                              return AlertDialog(
                                title: const Text("Remove user"),
                                content: const Text(
                                    "Are you sure you want to remove this user from the group?"),
                                actions: [
                                  TextButton(
                                      onPressed: () {
                                        Navigator.of(context).pop(false);
                                      },
                                      style: textButtonStyle(context),
                                      child: const Text("Cancel")),
                                  TextButton(
                                    onPressed: () {
                                      coreClient
                                          .removeUserFromConversation(
                                              widget.conversation.id,
                                              widget.username)
                                          .then((value) => {
                                                Navigator.of(context).pop(true)
                                              });
                                    },
                                    style: textButtonStyle(context),
                                    child: const Text("Remove user"),
                                  ),
                                ],
                              );
                            },
                          );
                          if (confirmed) {
                            Navigator.of(context).pop(true);
                          }
                        },
                        child: const Text("Remove user")),
                  )
                : Container(),
          ],
        ),
      ),
    );
  }
}
