// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/widgets/widgets.dart';

import 'connection_details.dart';
import 'conversation_details_cubit.dart';
import 'group_details.dart';

/// Container for [ConversationDetailsScreenView]
///
/// Wraps the screen with required providers.
class ConversationDetailsScreen extends StatelessWidget {
  const ConversationDetailsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const ConversationDetailsScreenView();
  }
}

/// Screen that shows details of a conversation
class ConversationDetailsScreenView extends StatelessWidget {
  const ConversationDetailsScreenView({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationType = context.select(
      (ConversationDetailsCubit cubit) =>
          cubit.state.conversation?.conversationType,
    );

    final loc = AppLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: const AppBarBackButton(),
        title: Text(loc.conversationDetailsScreen_title),
      ),
      body: SafeArea(
        child: switch (conversationType) {
          UiConversationType_HandleConnection() ||
          UiConversationType_Connection() => const ConnectionDetails(),
          UiConversationType_Group() => const GroupDetails(),
          null => Center(
            child: Text(loc.conversationDetailsScreen_unknownConversation),
          ),
        },
      ),
    );
  }
}
