// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/theme/theme.dart';
import 'package:provider/provider.dart';

import 'display_message_tile.dart';
import 'message_cubit.dart';
import 'text_message_tile.dart';

class ConversationTile extends StatelessWidget {
  const ConversationTile({super.key});

  @override
  Widget build(BuildContext context) {
    final (message, neighbors, timestamp) = context.select(
      (MessageCubit cubit) => (
        cubit.state.message?.message,
        cubit.state.message?.neighbors,
        cubit.state.message?.timestamp,
      ),
    );

    if (message == null || timestamp == null) {
      return const SizedBox.shrink();
    }

    return ListTile(
      contentPadding: EdgeInsets.symmetric(horizontal: Spacings.s),
      dense: true,
      visualDensity: VisualDensity(horizontal: 0, vertical: -4),
      minVerticalPadding: 0,
      title: Container(
        alignment: AlignmentDirectional.centerStart,
        child: switch (message) {
          UiMessage_ContentFlight(field0: final contentFlight) =>
            TextMessageTile(contentFlight, timestamp, neighbors: neighbors),
          UiMessage_Display(field0: final display) =>
            DisplayMessageTile(display, timestamp),
          UiMessage_Unsent(field0: final unsent) => Text(
              "⚠️ UNSENT MESSAGE ⚠️ $unsent",
              style: const TextStyle(color: Colors.red)),
        },
      ),
      selected: false,
    );
  }
}
