// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// New widget that shows conversation details
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/navigation/navigation.dart';
import 'package:prototype/styles.dart';
import 'package:provider/provider.dart';

// Constant for padding between the elements
const double _padding = 32;

class GroupDetails extends StatefulWidget {
  final UiConversationDetails conversation;

  const GroupDetails({super.key, required this.conversation});

  @override
  State<GroupDetails> createState() => _GroupDetailsState();
}

class _GroupDetailsState extends State<GroupDetails> {
  Uint8List? avatar;
  List<String> members = [];

  @override
  void initState() {
    super.initState();
    fetchMembers();
  }

  Future<void> fetchMembers() async {
    // Fetch member list from the core client
    members = await context.coreClient.getMembers(widget.conversation.id);
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          const SizedBox(height: _padding),
          UserAvatar(
              size: 64,
              image: widget.conversation.attributes.conversationPictureOption,
              username: widget.conversation.conversationType.when(
                  unconfirmedConnection: (e) => e,
                  connection: (e) => e,
                  group: () => widget.conversation.attributes.title),
              onPressed: () async {
                // Image picker
                final ImagePicker picker = ImagePicker();
                // Pick an image.
                final XFile? image =
                    await picker.pickImage(source: ImageSource.gallery);
                image?.readAsBytes().then((value) {
                  setState(() {
                    avatar = value;
                    context.coreClient.user.setConversationPicture(
                        conversationId: widget.conversation.id,
                        conversationPicture: value);
                  });
                });
              }),
          const SizedBox(height: _padding),
          Text(
            widget.conversation.conversationType.when(
                unconfirmedConnection: (e) => e,
                connection: (e) => e,
                group: () => widget.conversation.attributes.title),
            style: labelStyle,
          ),
          const SizedBox(height: _padding),
          Text(
            widget.conversation.conversationType.when(
                unconfirmedConnection: (e) => 'Pending connection request',
                connection: (e) => '1:1 conversation',
                group: () => 'Group conversation'),
            style: labelStyle,
          ),
          const SizedBox(height: _padding),
          Expanded(
            child: Container(
              constraints: const BoxConstraints(minWidth: 100, maxWidth: 600),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: _padding),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      "Members",
                      style: boldLabelStyle,
                    ),
                    Expanded(
                      child: ListView.builder(
                        itemCount: members.length,
                        itemBuilder: (context, index) {
                          return ListTile(
                            leading: FutureUserAvatar(
                              size: 24,
                              profile: context.coreClient.user
                                  .userProfile(userName: members[index]),
                            ),
                            title: Text(
                              members[index],
                              style: labelStyle,
                              overflow: TextOverflow.ellipsis,
                            ),
                            trailing: const Icon(Icons.more_horiz),
                            onTap: () {
                              context
                                  .read<NavigationCubit>()
                                  .openMemberDetails(members[index]);
                            },
                          );
                        },
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(height: _padding),
          OutlinedButton(
              onPressed: () {
                context.read<NavigationCubit>().openAddMembers();
              },
              child: const Text("Add members")),
          const SizedBox(height: _padding),
        ],
      ),
    );
  }
}
