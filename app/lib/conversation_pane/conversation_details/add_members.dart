// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'dart:collection';

import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';

class AddMembers extends StatefulWidget {
  const AddMembers({super.key});

  @override
  State<AddMembers> createState() => _AddMembersState();
}

class _AddMembersState extends State<AddMembers> {
  List<UiContact> contacts = [];
  HashSet<String> selectedContacts = HashSet();

  @override
  void initState() {
    super.initState();
    getContacts();
  }

  getContacts() async {
    final contacts = await context.coreClient.getContacts();
    setState(() {
      this.contacts = contacts;
    });
  }

  void conversationListener(UiConversationDetails conversation) async {
    Navigator.of(context).pop();
    return;
  }

  void toggleContactSelection(UiContact contact) {
    if (selectedContacts.contains(contact.userName)) {
      selectedContacts.remove(contact.userName);
    } else {
      selectedContacts.add(contact.userName);
    }
  }

  @override
  Widget build(BuildContext context) {
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
                          onChanged: (bool? value) {
                            setState(() {
                              toggleContactSelection(contact);
                            });
                          },
                        ),
                        onTap: () {
                          setState(() {
                            toggleContactSelection(contact);
                          });
                        },
                      );
                    },
                  ),
                ),
                _AddMembersButton(selectedContacts: selectedContacts),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _AddMembersButton extends StatelessWidget {
  const _AddMembersButton({
    required this.selectedContacts,
  });

  final HashSet<String> selectedContacts;

  @override
  Widget build(BuildContext context) {
    final conversationId = context.select(
      (NavigationCubit cubit) => cubit.state.conversationId,
    );
    var coreClient = context.coreClient;
    final isEnabled = selectedContacts.isNotEmpty;

    return OutlinedButton(
      onPressed: isEnabled
          ? () {
              _addContacts(coreClient, conversationId!);
              Navigator.of(context).pop(true);
            }
          : null,
      style: buttonStyle(context, isEnabled),
      child: const Text("Add member(s)"),
    );
  }

  _addContacts(CoreClient coreClient, ConversationId conversationId) async {
    for (var contact in selectedContacts) {
      await coreClient.addUserToConversation(conversationId, contact);
    }
  }
}
