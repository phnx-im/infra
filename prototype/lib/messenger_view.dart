// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/pane.dart';
import 'package:prototype/conversation_pane/conversation_pane.dart';
import 'package:prototype/styles.dart';

void main() {
  runApp(const MessengerView());
}

// Combined navigator
void pushToNavigator(BuildContext context, Widget widget) {
  Navigator.of(context).push(MaterialPageRoute(builder: (context) => widget));
}

class MessengerView extends StatelessWidget {
  const MessengerView({super.key});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (isSmallScreen(context)) {
          return const ConversationView();
        } else {
          return const Scaffold(
            body: Row(
              children: [
                SizedBox(
                  width: 300,
                  child: ConversationView(),
                ),
                Expanded(
                  child: ConversationPane(),
                ),
              ],
            ),
          );
        }
      },
    );
  }
}
