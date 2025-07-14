// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/message_list/message_list.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/user_avatar.dart';

import 'conversation_details_cubit.dart';

class ConversationScreen extends StatelessWidget {
  const ConversationScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    if (conversationId == null) {
      return const _EmptyConversationPane();
    }

    return MultiBlocProvider(
      providers: [
        BlocProvider(
          // rebuilds the cubit when a different conversation is selected
          key: ValueKey("message-list-cubit-$conversationId"),
          create:
              (context) => MessageListCubit(
                userCubit: context.read<UserCubit>(),
                conversationId: conversationId,
              ),
        ),
      ],
      child: const ConversationScreenView(),
    );
  }
}

class _EmptyConversationPane extends StatelessWidget {
  const _EmptyConversationPane();

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return Center(
      child: Text(
        style: Theme.of(
          context,
        ).textTheme.labelMedium?.copyWith(color: colorDMB),
        loc.conversationScreen_emptyConversation,
      ),
    );
  }
}

class ConversationScreenView extends StatelessWidget {
  const ConversationScreenView({
    super.key,
    this.createMessageCubit = MessageCubit.new,
  });

  final MessageCubitCreate createMessageCubit;

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );

    if (conversationId == null) {
      return const _EmptyConversationPane();
    }

    return Scaffold(
      body: Column(
        children: [
          const _ConversationHeader(),
          Expanded(
            child: MessageListView(createMessageCubit: createMessageCubit),
          ),
          const MessageComposer(),
        ],
      ),
    );
  }
}

class _ConversationHeader extends StatelessWidget {
  const _ConversationHeader();

  @override
  Widget build(BuildContext context) {
    final conversationTitle = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.conversation?.title,
    );

    final conversationPicture = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.conversation?.picture,
    );

    return Container(
      padding: EdgeInsets.only(
        top:
            context.responsiveScreenType == ResponsiveScreenType.mobile
                ? kToolbarHeight
                : Spacings.xxs,
        bottom: Spacings.xxs,
        left: Spacings.xs,
        right: Spacings.xs,
      ),
      child: Container(
        color: Colors.white,
        height: Spacings.l,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            context.responsiveScreenType == ResponsiveScreenType.mobile
                ? const _BackButton()
                : const SizedBox.shrink(),
            Row(
              spacing: Spacings.xs,
              children: [
                UserAvatar(
                  displayName: conversationTitle ?? "",
                  image: conversationPicture,
                  size: Spacings.m,
                  onPressed: () {
                    context.read<NavigationCubit>().openConversationDetails();
                  },
                ),
                Text(
                  conversationTitle ?? "",
                  style: const TextStyle(
                    color: Colors.black,
                    fontSize: 14,
                    fontVariations: variationBold,
                  ),
                ),
              ],
            ),
            conversationTitle != null
                ? const _DetailsButton()
                : const SizedBox.shrink(),
          ],
        ),
      ),
    );
  }
}

class _DetailsButton extends StatelessWidget {
  const _DetailsButton();

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.more_horiz, size: 26),
      color: Colors.black,
      padding: const EdgeInsets.symmetric(horizontal: Spacings.xs),
      hoverColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      onPressed: () {
        context.read<NavigationCubit>().openConversationDetails();
      },
    );
  }
}

class _BackButton extends StatelessWidget {
  const _BackButton();

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.arrow_back, size: 26),
      padding: const EdgeInsets.symmetric(horizontal: Spacings.xs),
      color: Colors.black,
      hoverColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      onPressed: () {
        context.read<NavigationCubit>().closeConversation();
      },
    );
  }
}
