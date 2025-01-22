// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list/conversation_list.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/theme/theme.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    const mobileLayout = ConversationListContainer();
    const desktopLayout = Row(
      children: [
        SizedBox(
          width: 300,
          child: ConversationListContainer(),
        ),
        Expanded(
          child: ConversationScreen(),
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
