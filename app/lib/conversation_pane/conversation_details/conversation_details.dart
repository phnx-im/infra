// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details

import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/conversation_list_pane/conversation_list_cubit.dart';
import 'package:prototype/conversation_pane/conversation_details/connection_details.dart';
import 'package:prototype/conversation_pane/conversation_details/group_details.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';

class ConversationDetailsScreen extends StatelessWidget {
  const ConversationDetailsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      create: (context) => ConversationListCubit(userCubit: context.read()),
      child: const ConversationDetailsView(),
    );
  }
}

class ConversationDetailsView extends StatelessWidget {
  const ConversationDetailsView({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );
    final conversation = context.select(
      (ConversationListCubit cubit) => conversationId != null
          ? cubit.state.conversations.firstWhereOrNull(
              (conversation) => conversation.id == conversationId)
          : null,
    );

    if (conversation == null) {
      return const SizedBox.shrink();
    }

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: appBarBackButton(context),
        title: const Text("Details"),
      ),
      body: switch (conversation.conversationType) {
        UiConversationType_UnconfirmedConnection() ||
        UiConversationType_Connection() =>
          ConnectionDetails(conversation: conversation),
        UiConversationType_Group() => GroupDetails(conversation: conversation),
      },
    );
  }
}
