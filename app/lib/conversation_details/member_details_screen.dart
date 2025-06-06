// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_details/conversation_details_cubit.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/l10n/l10n.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';
import 'package:provider/provider.dart';

// Constant for padding between the elements
const double _padding = 32;

class MemberDetailsScreen extends StatelessWidget {
  const MemberDetailsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final loc = AppLocalizations.of(context);

    final (conversationId, memberId) = context.select(
      (NavigationCubit cubit) => switch (cubit.state) {
        NavigationState_Intro(:final screens) =>
          throw StateError(loc.memberDetailsScreen_error),
        NavigationState_Home(
          home: HomeNavigationState(
            conversationId: final conversationId,
            memberDetails: final memberId,
          ),
        ) =>
          (conversationId, memberId),
      },
    );

    final ownUserId = context.select((UserCubit cubit) => cubit.state.userId);
    final isSelf = memberId == ownUserId;

    final profile = context.select(
      (UsersCubit cubit) => cubit.state.profile(userId: memberId),
    );

    final roomState = context.select(
      (ConversationDetailsCubit cubit) => cubit.state.roomState,
    );

    if (conversationId == null || memberId == null || roomState == null) {
      return const SizedBox.shrink();
    }

    final canKick = roomState.canKick(target: memberId);

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: const AppBarBackButton(),
        title: Text(loc.memberDetailsScreen_title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Column(
              mainAxisAlignment: MainAxisAlignment.start,
              children: [
                const SizedBox(height: _padding),
                UserAvatar(
                  displayName: profile.displayName,
                  image: profile.profilePicture,
                  size: 64,
                ),
                const SizedBox(height: _padding),
                Text(
                  style: Theme.of(context).textTheme.labelMedium,
                  profile.displayName,
                ),
                const SizedBox(height: _padding),
              ],
            ),
            // Show the remove user button if the user is not the current user and has kicking rights
            if (!isSelf && canKick)
              Padding(
                padding: const EdgeInsets.all(_padding),
                child: OutlinedButton(
                  onPressed: () async {
                    bool confirmed = await showDialog(
                      context: context,
                      builder: (BuildContext context) {
                        return AlertDialog(
                          title: Text(loc.removeUserDialog_title),
                          content: Text(loc.removeUserDialog_content),
                          actions: [
                            TextButton(
                              onPressed: () {
                                Navigator.of(context).pop(false);
                              },
                              style: textButtonStyle(context),
                              child: Text(loc.removeUserDialog_cancel),
                            ),
                            TextButton(
                              onPressed: () async {
                                await context
                                    .read<UserCubit>()
                                    .removeUserFromConversation(
                                      conversationId,
                                      memberId,
                                    );
                                if (context.mounted) {
                                  Navigator.of(context).pop(true);
                                }
                              },
                              style: textButtonStyle(context),
                              child: Text(loc.removeUserDialog_removeUser),
                            ),
                          ],
                        );
                      },
                    );
                    if (confirmed && context.mounted) {
                      Navigator.of(context).pop(true);
                    }
                  },
                  child: Text(loc.removeUserButton_text),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
