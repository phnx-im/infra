import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:prototype/conversation_list_pane/conversation_list.dart';
import 'package:prototype/conversation_list_pane/footer.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/settings/developer.dart';
import 'package:prototype/settings/user.dart';
import 'package:prototype/styles.dart';

class ConversationView extends StatefulWidget {
  const ConversationView({super.key});

  @override
  State<ConversationView> createState() => _ConversationViewState();
}

class _ConversationViewState extends State<ConversationView> {
  String? displayName = coreClient.ownProfile.displayName;
  Uint8List? profilePicture = coreClient.ownProfile.profilePictureOption;

  @override
  void initState() {
    super.initState();
    // Listen for changes to the user's profile picture
    coreClient.onOwnProfileUpdate.listen((profile) {
      if (mounted) {
        setState(() {
          profilePicture = profile.profilePictureOption;
          displayName = profile.displayName;
        });
      }
    });
  }

  @override
  Widget build(
    BuildContext context,
  ) {
    return Container(
      decoration: const BoxDecoration(
        shape: BoxShape.rectangle,
        border: Border(
          right: BorderSide(
            width: 1,
            color: colorGreyLight,
          ),
        ),
      ),
      child: Scaffold(
        backgroundColor: convPaneBackgroundColor,
        appBar: PreferredSize(
          // Leave some space for macOS windows controls
          preferredSize: isPointer()
              ? const Size.fromHeight(kToolbarHeight + 30)
              : const Size.fromHeight(kToolbarHeight),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 20.0),
                child: AppBar(
                  title: Column(
                    children: [
                      Text(
                        displayName ?? "",
                        style: const TextStyle(
                          color: colorDMB,
                          fontVariations: variationBold,
                          fontSize: 13,
                          letterSpacing: -0.2,
                        ),
                      ),
                      const SizedBox(height: 5),
                      Text(
                        coreClient.username,
                        style: const TextStyle(
                          color: colorDMB,
                          fontSize: 10,
                          fontVariations: variationRegular,
                          letterSpacing: -0.2,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                  actions: [
                    Padding(
                      padding: const EdgeInsets.only(right: 8.0),
                      child: IconButton(
                        onPressed: () {
                          Navigator.push(
                            context,
                            MaterialPageRoute(
                              builder: (context) =>
                                  const DeveloperSettingsScreen(),
                            ),
                          );
                        },
                        hoverColor: Colors.transparent,
                        focusColor: Colors.transparent,
                        splashColor: Colors.transparent,
                        highlightColor: Colors.transparent,
                        icon: const Icon(
                          Icons.settings,
                          size: 20,
                          color: colorDMB,
                        ),
                      ),
                    )
                  ],
                  backgroundColor: convPaneBackgroundColor,
                  elevation: 0,
                  scrolledUnderElevation: 0,
                  leading: Padding(
                    padding: const EdgeInsets.only(left: 12.0),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      crossAxisAlignment: CrossAxisAlignment.center,
                      children: [
                        UserAvatar(
                          size: 32,
                          username: coreClient.username,
                          image: profilePicture,
                          onPressed: () {
                            Navigator.push(
                              context,
                              MaterialPageRoute(
                                builder: (context) =>
                                    const UserSettingsScreen(),
                              ),
                            );
                          },
                        )
                      ],
                    ),
                  ),
                ),
              )
            ],
          ),
        ),
        body: Container(
          color: colorDMBSuperLight,
          child: const Column(
            children: [
              SizedBox(height: 10),
              Expanded(
                child: ConversationList(),
              ),
              Footer(),
            ],
          ),
        ),
      ),
    );
  }
}
