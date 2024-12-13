// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/conversation_list.dart';
import 'package:prototype/conversation_list_pane/footer.dart';
import 'package:prototype/conversation_list_pane/top.dart';
import 'package:prototype/styles.dart';
import 'package:prototype/theme/theme.dart';

class ConversationView extends StatelessWidget {
  const ConversationView({super.key});

  @override
  Widget build(BuildContext context) {
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
      child: const Scaffold(
        backgroundColor: convPaneBackgroundColor,
        body: Column(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            ConversationListTop(),
            SizedBox(height: Spacings.s),
            Expanded(child: ConversationList()),
            ConversationListFooter(),
          ],
        ),
      ),
    );
  }
}
