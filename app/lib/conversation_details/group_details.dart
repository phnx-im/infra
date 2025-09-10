// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:air/core/core.dart';
import 'package:air/navigation/navigation.dart';
import 'package:air/theme/theme.dart';
import 'package:air/user/user.dart';
import 'package:air/widgets/widgets.dart';
import 'package:provider/provider.dart';

import 'conversation_details_cubit.dart';

/// Shows group chat details
class GroupDetails extends StatelessWidget {
  const GroupDetails({super.key});

  @override
  Widget build(BuildContext context) {
    final (chat, members) = context.select((ConversationDetailsCubit cubit) {
      final state = cubit.state;
      return (state.chat, state.members);
    });

    if (chat == null) {
      return const SizedBox.shrink();
    }

    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Spacings.l),
        child: Column(
          spacing: Spacings.l,
          children: [
            UserAvatar(
              size: 128,
              image: chat.picture,
              displayName: chat.title,
              onPressed: () async {
                final conversationDetailsCubit =
                    context.read<ConversationDetailsCubit>();
                // Image picker
                final ImagePicker picker = ImagePicker();
                // Pick an image.
                final XFile? image = await picker.pickImage(
                  source: ImageSource.gallery,
                );
                final bytes = await image?.readAsBytes();
                conversationDetailsCubit.setChatPicture(bytes: bytes);
              },
            ),
            Text(chat.title, style: Theme.of(context).textTheme.bodyLarge),
            Text(
              chat.chatType.description,
              style: Theme.of(context).textTheme.bodyMedium,
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
                        style: Theme.of(context).textTheme.labelLarge,
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
          ],
        ),
      ),
    );
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
        size: Spacings.l,
      ),
      title: Text(
        profile.displayName,
        style: Theme.of(context).textTheme.bodyMedium,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: const Icon(Icons.more_horiz),
      onTap: () {
        context.read<NavigationCubit>().openMemberDetails(memberId);
      },
    );
  }
}
