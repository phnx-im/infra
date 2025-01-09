// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';

import 'connection_details.dart';
import 'conversation_details_cubit.dart';
import 'group_details.dart';

/// Container for [ConversationDetailsScreen]
///
/// Wraps the screen with required providers.
class ConversationDetailsContainer extends StatelessWidget {
  const ConversationDetailsContainer({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId =
        context.select((NavigationCubit cubit) => cubit.state.conversationId);
    if (conversationId == null) {
      throw StateError("an active conversation is obligatory");
    }

    return BlocProvider(
      key: ValueKey(conversationId),
      create: (context) => ConversationDetailsCubit(
        userCubit: context.read(),
        conversationId: conversationId,
      ),
      child: ConversationDetailsScreen(),
    );
  }
}

/// Screen that shows details of a conversation
class ConversationDetailsScreen extends StatelessWidget {
  const ConversationDetailsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationType = context.select((ConversationDetailsCubit cubit) =>
        cubit.state.conversation?.conversationType);

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: appBarBackButton(context),
        title: const Text("Details"),
      ),
      body: switch (conversationType) {
        UiConversationType_UnconfirmedConnection() ||
        UiConversationType_Connection() =>
          const ConnectionDetails(),
        UiConversationType_Group() => const GroupDetails(),
        null => Center(child: const Text("Unknown conversation")),
      },
    );
  }
}
