// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:air/core/core.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:air/l10n/l10n.dart';
import 'package:air/message_list/message_list.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/ui/typography/font_size.dart';
import 'package:air/user/user.dart';
import 'package:air/widgets/user_avatar.dart';

import 'chat_details_cubit.dart';
import 'delete_chat_button.dart';
import 'report_spam_button.dart';
import 'unblock_contact_button.dart';

class ChatScreen extends StatelessWidget {
  const ChatScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final chatId = context.select(
      (NavigationCubit cubit) => cubit.state.openChatId,
    );

    if (chatId == null) {
      return const _EmptyChatPane();
    }

    return MultiBlocProvider(
      providers: [
        BlocProvider(
          // rebuilds the cubit when a different chat is selected
          key: ValueKey("message-list-cubit-$chatId"),
          create:
              (context) => MessageListCubit(
                userCubit: context.read<UserCubit>(),
                chatId: chatId,
              ),
        ),
      ],
      child: const ChatScreenView(),
    );
  }
}

class _EmptyChatPane extends StatelessWidget {
  const _EmptyChatPane();

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    return Center(
      child: Text(
        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
          color: CustomColorScheme.of(context).text.tertiary,
        ),
        loc.chatScreen_emptyChat,
      ),
    );
  }
}

class ChatScreenView extends StatelessWidget {
  const ChatScreenView({super.key, this.createMessageCubit = MessageCubit.new});

  final MessageCubitCreate createMessageCubit;

  @override
  Widget build(BuildContext context) {
    final chatId = context.select(
      (NavigationCubit cubit) => cubit.state.chatId,
    );
    if (chatId == null) {
      return const _EmptyChatPane();
    }

    final (blockedUserId, blockedUserDisplayName) = context.select((
      ChatDetailsCubit cubit,
    ) {
      final chat = cubit.state.chat;
      return switch (chat?.status) {
        UiChatStatus_Blocked() => (chat?.userId, chat?.displayName),
        _ => (null, null),
      };
    });

    return Scaffold(
      body: Container(
        decoration: BoxDecoration(
          color: CustomColorScheme.of(context).backgroundBase.primary,
        ),
        child: Column(
          children: [
            const _ChatHeader(),
            Expanded(
              child: MessageListView(createMessageCubit: createMessageCubit),
            ),
            blockedUserId == null || blockedUserDisplayName == null
                ? const MessageComposer()
                : _BlockedChatFooter(
                  chatId: chatId,
                  userId: blockedUserId,
                  displayName: blockedUserDisplayName,
                ),
          ],
        ),
      ),
    );
  }
}

class _ChatHeader extends StatelessWidget {
  const _ChatHeader();

  @override
  Widget build(BuildContext context) {
    final title = context.select(
      (ChatDetailsCubit cubit) => cubit.state.chat?.title,
    );

    final image = context.select(
      (ChatDetailsCubit cubit) => cubit.state.chat?.picture,
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
                  displayName: title ?? "",
                  image: image,
                  size: Spacings.l,
                  onPressed: () {
                    context.read<NavigationCubit>().openChatDetails();
                  },
                ),
                Text(
                  title ?? "",
                  style: TextStyle(
                    fontSize: LabelFontSize.base.size,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
            title != null ? const _DetailsButton() : const SizedBox.shrink(),
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
        context.read<NavigationCubit>().openChatDetails();
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
        context.read<NavigationCubit>().closeChat();
      },
    );
  }
}

class _BlockedChatFooter extends StatelessWidget {
  const _BlockedChatFooter({
    required this.chatId,
    required this.userId,
    required this.displayName,
  });

  final ChatId chatId;
  final UiUserId userId;
  final String displayName;

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);
    final buttonWidthConstraints =
        isPointer() ? const BoxConstraints(minWidth: 100) : null;
    return Container(
      constraints: isPointer() ? const BoxConstraints(maxWidth: 800) : null,
      padding: const EdgeInsets.all(Spacings.s),
      child: Column(
        children: [
          Text(loc.blockedChatFooter_message(displayName)),
          const SizedBox(height: Spacings.s),
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Container(
                constraints: buttonWidthConstraints,
                child: DeleteChatButton(chatId: chatId),
              ),
              const SizedBox(width: Spacings.s),
              Container(
                constraints: buttonWidthConstraints,
                child: ReportSpamButton(userId: userId),
              ),
              const SizedBox(width: Spacings.s),
              Container(
                constraints: buttonWidthConstraints,
                child: UnblockContactButton(
                  userId: userId,
                  displayName: displayName,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
