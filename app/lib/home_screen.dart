// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:prototype/conversation_list/conversation_list.dart';
import 'package:prototype/conversation_details/conversation_details.dart';
import 'package:prototype/theme/theme.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  bool _fontVariationsEnabled = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: SingleChildScrollView(
          child: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                CheckboxListTile(
                  title: const Text('Enable Font Variations'),
                  value: _fontVariationsEnabled,
                  onChanged: (enabled) => setState(() {
                    _fontVariationsEnabled = enabled ?? false;
                  }),
                  controlAffinity: ListTileControlAffinity.leading,
                ),
                Text("Embedded Inter, letter spacing: -0.2"),
                const SizedBox(height: Spacings.xxs),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: FontWeight.values
                      .map(
                        (weight) => Text(
                          'This text has weight $weight',
                          style: TextStyle(fontWeight: weight, fontVariations: [
                            if (_fontVariationsEnabled)
                              FontVariation(
                                  'wght', ((weight.index + 1) * 100).toDouble())
                          ]),
                        ),
                      )
                      .toList(),
                ),
                Divider(),
                Text("Google InterTight"),
                const SizedBox(height: Spacings.xxs),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: FontWeight.values
                      .map(
                        (weight) => Text(
                          'This text has weight $weight',
                          style: GoogleFonts.interTight(fontWeight: weight),
                        ),
                      )
                      .toList(),
                ),
                Divider(),
                Text("Google Inter"),
                const SizedBox(height: Spacings.xxs),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: FontWeight.values
                      .map(
                        (weight) => Text(
                          'This text has weight $weight',
                          style: GoogleFonts.inter(fontWeight: weight),
                        ),
                      )
                      .toList(),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    const mobileLayout = ConversationListContainer();
    const desktopLayout = HomeScreenDesktopLayout(
      conversationList: ConversationListContainer(),
      conversation: ConversationScreen(),
    );
    return const ResponsiveScreen(
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
    return Row(
      children: [
        SizedBox(
          width: 300,
          child: conversationList,
        ),
        Expanded(
          child: conversation,
        ),
      ],
    );
  }
}
