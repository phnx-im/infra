// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'package:provider/provider.dart';

import 'display_message_tile.dart';
import 'message_cubit.dart';
import 'text_message_tile.dart';

class ConversationTile extends StatelessWidget {
  const ConversationTile({super.key});

  @override
  Widget build(BuildContext context) {
    final (message, timestamp, position) = context.select(
      (MessageCubit cubit) => (
        cubit.state.message.message,
        cubit.state.message.timestamp,
        cubit.state.message.position,
      ),
    );

    return ListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: Spacings.s),
      dense: true,
      visualDensity: const VisualDensity(horizontal: 0, vertical: -4),
      minVerticalPadding: 0,
      title: Container(
        alignment: AlignmentDirectional.centerStart,
        child: switch (message) {
          UiMessage_Content(field0: final content) => TextMessageTile(
            contentMessage: content,
            timestamp: timestamp,
            flightPosition: position,
          ),
          UiMessage_Display(field0: final display) => DisplayMessageTile(
            display,
            timestamp,
          ),
        },
      ),
      selected: false,
    );
  }
}
