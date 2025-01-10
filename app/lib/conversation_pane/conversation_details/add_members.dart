// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';

import 'add_members_cubit.dart';

class AddMembers extends StatelessWidget {
  const AddMembers({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      create: (context) =>
          AddMembersCubit(coreClient: context.read())..loadContacts(),
      child: const AddMembersView(),
    );
  }
}

class AddMembersView extends StatelessWidget {
  const AddMembersView({super.key});

  @override
  Widget build(BuildContext context) {
    final (contacts, selectedContacts) = context.select(
      (AddMembersCubit cubit) => (
        cubit.state.contacts,
        cubit.state.selectedContacts,
      ),
    );

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        scrolledUnderElevation: 0,
        leading: appBarBackButton(context),
        title: const Text("Add members"),
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
                      return ListTile(
                        leading: FutureUserAvatar(
                          profile: () => context.coreClient.user
                              .userProfile(userName: contact.userName),
                        ),
                        title: Text(
                          contact.userName,
                          style: labelStyle,
                          overflow: TextOverflow.ellipsis,
                        ),
                        trailing: Checkbox(
                          value: selectedContacts.contains(contact.userName),
                          checkColor: colorDMB,
                          fillColor: WidgetStateProperty.all(colorGreyLight),
                          focusColor: Colors.transparent,
                          hoverColor: Colors.transparent,
                          overlayColor:
                              WidgetStateProperty.all(Colors.transparent),
                          side: BorderSide.none,
                          shape: const CircleBorder(),
                          onChanged: (bool? value) => context
                              .read<AddMembersCubit>()
                              .toggleContact(contact),
                        ),
                        onTap: () => context
                            .read<AddMembersCubit>()
                            .toggleContact(contact),
                      );
                    },
                  ),
                ),
                OutlinedButton(
                  onPressed: selectedContacts.isNotEmpty
                      ? () async {
                          final navigation = context.read<NavigationCubit>();
                          final conversationId =
                              navigation.state.conversationId;
                          if (conversationId == null) {
                            throw StateError(
                                "an active conversation is obligatory");
                          }
                          await context
                              .read<AddMembersCubit>()
                              .addContacts(conversationId);
                          navigation.pop();
                        }
                      : null,
                  style: buttonStyle(context, selectedContacts.isNotEmpty),
                  child: const Text("Add member(s)"),
                )
              ],
            ),
          ),
        ),
      ),
    );
  }
}
