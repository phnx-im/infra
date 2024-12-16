// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/core_extension.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';

// Constant for padding between the elements
const double _padding = 32;

class ConnectionDetails extends StatelessWidget {
  final UiConversationDetails conversation;

  const ConnectionDetails({super.key, required this.conversation});

  @override
  Widget build(BuildContext context) {
    final coreClient = context.coreClient;
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          const SizedBox(height: _padding),
          FutureUserAvatar(
            size: 64,
            profile: () => conversation.userProfile(coreClient),
          ),
          const SizedBox(height: _padding),
          Text(
            conversation.title,
            style: labelStyle,
          ),
          const SizedBox(height: _padding),
          Text(
            conversation.conversationType.description,
            style: labelStyle,
          ),
          const SizedBox(height: _padding),
        ],
      ),
    );
  }
}
