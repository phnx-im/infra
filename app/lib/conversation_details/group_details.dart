// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/util/dialog.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'conversation_details_cubit.dart';

/// Shows conversation details
class GroupDetails extends StatelessWidget {
  const GroupDetails({super.key});

  @override
  Widget build(BuildContext context) {
    final (conversation, members) = context.select((
      ConversationDetailsCubit cubit,
    ) {
      final state = cubit.state;
      return (state.conversation, state.members);
    });

    if (conversation == null) {
      return const SizedBox.shrink();
    }

    return Align(
      alignment: Alignment.topCenter,
      child: Container(
        constraints: isPointer() ? const BoxConstraints(maxWidth: 800) : null,
        padding: const EdgeInsets.symmetric(vertical: Spacings.l),
        child: Column(
          spacing: Spacings.s,
          children: [
            UserAvatar(
              size: 100,
              image: conversation.picture,
              displayName: conversation.title,
              onPressed: () => _selectAvatar(context, conversation.id),
            ),
            Text(
              conversation.title,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            Text(
              conversation.conversationType.description,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            Expanded(
              child: Container(
                constraints: const BoxConstraints(minWidth: 100, maxWidth: 600),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: Spacings.l),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        "Members",
                        style: Theme.of(
                          context,
                        ).textTheme.labelMedium?.merge(VariableFontWeight.bold),
                      ),
                      Expanded(
                        child: ListView.builder(
                          itemCount: members.length,
                          itemBuilder: (context, index) {
                            final memberId = members[index];
                            return _MemberTile(memberId: memberId);
                          },
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            OutlinedButton(
              onPressed: () {
                context.read<NavigationCubit>().openAddMembers();
              },
              child: const Text("Add members"),
            ),
            Divider(color: Theme.of(context).hintColor),
            OutlinedButton(
              style: dangerButtonStyle(context),
              onPressed: () => _leave(context, conversation.id),
              child: const Text("Leave"),
            ),
            OutlinedButton(
              style: dangerButtonStyle(context),
              onPressed: () => _delete(context, conversation.id),
              child: const Text("Delete"),
            ),
          ],
        ),
      ),
    );
  }

  void _selectAvatar(BuildContext context, ConversationId id) async {
    final conversationDetailsCubit = context.read<ConversationDetailsCubit>();
    final ImagePicker picker = ImagePicker();
    final XFile? image = await picker.pickImage(source: ImageSource.gallery);
    if (image == null) {
      return;
    }
    final bytes = await image.readAsBytes();
    conversationDetailsCubit.setConversationPicture(bytes: bytes);
  }

  void _leave(BuildContext context, ConversationId id) async {
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    if (await showConfirmationDialog(
      context,
      title: "Leave conversation",
      message: "Are you sure you want to leave this conversation?",
      positiveButtonText: "Leave",
      negativeButtonText: "Cancel",
    )) {
      userCubit.leaveConversation(id);
      navigationCubit.closeConversation();
    }
  }

  void _delete(BuildContext context, ConversationId id) async {
    final userCubit = context.read<UserCubit>();
    final navigationCubit = context.read<NavigationCubit>();
    if (await showConfirmationDialog(
      context,
      title: "Leave conversation",
      message:
          "Are you sure you want to delete this conversation? "
          "The message history will be also deleted.",
      positiveButtonText: "Delete",
      negativeButtonText: "Cancel",
    )) {
      userCubit.deleteConversation(id);
      navigationCubit.closeConversation();
    }
  }
}

class _MemberTile extends StatelessWidget {
  const _MemberTile({required this.memberId});

  final UiUserId memberId;

  @override
  Widget build(BuildContext context) {
    final profile = context.select(
      (UsersCubit cubit) => cubit.state.profile(userId: memberId),
    );

    return ListTile(
      leading: UserAvatar(
        displayName: profile.displayName,
        image: profile.profilePicture,
      ),
      title: Text(
        profile.displayName,
        style: Theme.of(context).textTheme.labelMedium,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: const Icon(Icons.more_horiz),
      onTap: () {
        context.read<NavigationCubit>().openMemberDetails(memberId);
      },
    );
  }
}
