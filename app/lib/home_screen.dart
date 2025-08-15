// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list/conversation_list.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/ui/colors/themes.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/util/resizable_panel.dart';
import 'package:provider/provider.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final mobileLayout = Scaffold(
      backgroundColor: CustomColorScheme.of(context).backgroundBase.primary,
      body: const ConversationListContainer(),
    );
    const desktopLayout = HomeScreenDesktopLayout(
      conversationList: ConversationListContainer(),
      conversation: ConversationScreen(),
    );
    return ResponsiveScreen(
      mobile: mobileLayout,
      tablet: desktopLayout,
      desktop: desktopLayout,
    );
  }
}

class HomeScreenDesktopLayout extends StatelessWidget {
  const HomeScreenDesktopLayout({
    required this.conversationList,
    required this.conversation,
    super.key,
  });

  final Widget conversationList;
  final Widget conversation;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: CustomColorScheme.of(context).backgroundBase.primary,
      body: Row(
        children: [
          ResizablePanel(
            initialWidth: context.read<UserSettingsCubit>().state.sidebarWidth,
            onResizeEnd: (width) => onResizeEnd(context, width),
            child: conversationList,
          ),
          Expanded(child: conversation),
        ],
      ),
    );
  }

  void onResizeEnd(BuildContext context, double panelWidth) {
    context.read<UserSettingsCubit>().setSidebarWidth(
      userCubit: context.read(),
      value: panelWidth,
    );
  }
}
