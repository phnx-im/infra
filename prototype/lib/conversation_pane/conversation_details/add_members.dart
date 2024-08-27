// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'dart:async';
import 'dart:collection';

import 'package:flutter/material.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';

class AddMembers extends StatefulWidget {
  final UiConversationDetails conversation;

  const AddMembers({super.key, required this.conversation});

  @override
  State<AddMembers> createState() => _AddMembersState();
}

class _AddMembersState extends State<AddMembers> {
  late StreamSubscription<UiConversationDetails> _conversationListener;
  List<UiContact> contacts = [];
  HashSet<String> selectedContacts = HashSet();
  bool isButtonEnabled = false;

  @override
  void initState() {
    super.initState();
    _conversationListener =
        coreClient.onConversationSwitch.listen(conversationListener);
    getContacts();
  }

  @override
  void dispose() {
    _conversationListener.cancel();
    super.dispose();
  }

  getContacts() async {
    contacts = await coreClient.getContacts();
    setState(() {});
  }

  addContacts() async {
    for (var contact in selectedContacts) {
      await coreClient.addUserToConversation(widget.conversation.id, contact);
    }
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
    setState(() {
      isButtonEnabled = selectedContacts.isNotEmpty;
    });
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
                          profile: coreClient.user
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
                OutlinedButton(
                  onPressed: () {
                    if (isButtonEnabled) {
                      addContacts();
                      Navigator.of(context).pop(true);
                    }
                  },
                  style: buttonStyle(context, isButtonEnabled),
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
