// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/pane.dart';
import 'package:prototype/conversation_pane/conversation_pane.dart';
import 'package:prototype/theme/theme.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    const mobileLayout = ConversationView();
    const desktopLayout = Row(
      children: [
        SizedBox(
          width: 300,
          child: ConversationView(),
        ),
        Expanded(
          child: ConversationPane(),
        ),
      ],
    );
    return const ResponsiveScreen(
      mobile: mobileLayout,
      tablet: desktopLayout,
      desktop: desktopLayout,
    );
  }
}
