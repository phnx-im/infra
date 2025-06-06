// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/l10n/app_localizations.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/theme/theme.dart';
import 'package:prototype/user/user.dart';
import 'package:prototype/widgets/widgets.dart';

import 'add_members_cubit.dart';

class AddMembersScreen extends StatelessWidget {
  const AddMembersScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      create: (context) {
        final userCubit = context.read<UserCubit>();
        final navigationCubit = context.read<NavigationCubit>();
        final conversationId = navigationCubit.state.conversationId;
        final contactsFuture =
            conversationId != null
                ? userCubit.addableContacts(conversationId)
                : Future.value(<UiContact>[]);

        return AddMembersCubit()..loadContacts(contactsFuture);
      },
      child: const AddMembersScreenView(),
    );
  }
}

class AddMembersScreenView extends StatelessWidget {
  const AddMembersScreenView({super.key});

  @override
  Widget build(BuildContext context) {
    final (contacts, selectedContacts) = context.select(
      (AddMembersCubit cubit) => (
        cubit.state.contacts,
        cubit.state.selectedContacts,
      ),
    );

    final loc = AppLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: const AppBarBackButton(),
        title: Text(loc.addMembersScreen_title),
      ),
      body: Padding(
        padding: const EdgeInsets.all(32.0),
        child: Center(
          child: Container(
            constraints: const BoxConstraints(minWidth: 100, maxWidth: 600),
            child: Column(
              children: [
                Expanded(
                  child: ListView.builder(
                    itemCount: contacts.length,
                    itemBuilder: (context, index) {
                      final contact = contacts[index];
                      return _MemberTile(
                        contact: contact,
                        selectedContacts: selectedContacts,
                      );
                    },
                  ),
                ),
                OutlinedButton(
                  onPressed:
                      selectedContacts.isNotEmpty
                          ? () async {
                            _addSelectedContacts(context, selectedContacts);
                          }
                          : null,
                  style: buttonStyle(context, selectedContacts.isNotEmpty),
                  child: Text(loc.addMembersScreen_addMembers),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _addSelectedContacts(
    BuildContext context,
    Set<UiUserId> selectedContacts,
  ) async {
    final navigationCubit = context.read<NavigationCubit>();
    final userCubit = context.read<UserCubit>();
    final conversationId = navigationCubit.state.conversationId;
    final loc = AppLocalizations.of(context);
    if (conversationId == null) {
      throw StateError(loc.addMembersScreen_error_noActiveConversation);
    }
    for (final userId in selectedContacts) {
      await userCubit.addUserToConversation(conversationId, userId);
    }
    navigationCubit.pop();
  }
}

class _MemberTile extends StatelessWidget {
  const _MemberTile({required this.contact, required this.selectedContacts});

  final UiContact contact;
  final Set<UiUserId> selectedContacts;

  @override
  Widget build(BuildContext context) {
    final profile = context.select(
      (ContactsCubit cubit) => cubit.state.profile(userId: contact.userId),
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
      trailing: Checkbox(
        value: selectedContacts.contains(contact.userId),
        checkColor: colorDMB,
        fillColor: WidgetStateProperty.all(colorGreyLight),
        focusColor: Colors.transparent,
        hoverColor: Colors.transparent,
        overlayColor: WidgetStateProperty.all(Colors.transparent),
        side: BorderSide.none,
        shape: const CircleBorder(),
        onChanged:
            (bool? value) =>
                context.read<AddMembersCubit>().toggleContact(contact),
      ),
      onTap: () => context.read<AddMembersCubit>().toggleContact(contact),
    );
  }
}
