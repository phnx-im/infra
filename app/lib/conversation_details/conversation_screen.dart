// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/core/core.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/message_list/message_list.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/ui/typography/font_size.dart';
import 'package:air/user/user.dart';
import 'package:air/widgets/user_avatar.dart';

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
        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
          color: CustomColorScheme.of(context).text.tertiary,
        ),
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
      body: Container(
        decoration: BoxDecoration(
          color: CustomColorScheme.of(context).backgroundBase.primary,
        ),
        child: Column(
          children: [
            const _ConversationHeader(),
            Expanded(
              child: MessageListView(createMessageCubit: createMessageCubit),
            ),
            const MessageComposer(),
          ],
        ),
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
      child: SizedBox(
        height: Spacings.xl,
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
                  size: Spacings.l,
                  onPressed: () {
                    context.read<NavigationCubit>().openConversationDetails();
                  },
                ),
                Text(
                  conversationTitle ?? "",
                  style: TextStyle(
                    fontSize: LabelFontSize.base.size,
                    fontWeight: FontWeight.w600,
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
      icon: const Icon(Icons.more_horiz, size: 32),
      color: CustomColorScheme.of(context).text.primary,
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
      color: CustomColorScheme.of(context).text.primary,
      hoverColor: Colors.transparent,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,
      onPressed: () {
        context.read<NavigationCubit>().closeConversation();
      },
    );
  }
}
