// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/conversation_pane/message_renderer.dart';
import 'package:prototype/core/api/types.dart';
import 'package:prototype/core_client.dart';
import 'package:prototype/elements.dart';
import 'package:prototype/styles.dart';
import 'tile_timestamp.dart';

class TextMessageTile extends StatefulWidget {
  final UiContentMessage contentMessage;
  final DateTime timestamp;
  const TextMessageTile(this.contentMessage, this.timestamp, {super.key});

  @override
  State<TextMessageTile> createState() => _TextMessageTileState();
}

class _TextMessageTileState extends State<TextMessageTile> {
  bool _hovering = false;
  UiUserProfile? profile;

  @override
  void initState() {
    super.initState();
    coreClient.user
        .userProfile(userName: widget.contentMessage.sender)
        .then((p) {
      if (mounted) {
        setState(() {
          profile = p;
        });
      }
    });
  }

  void onEnter(PointerEvent e) {
    setState(() {
      _hovering = true;
    });
  }

  void onExit(PointerEvent e) {
    setState(() {
      _hovering = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: onEnter,
      onExit: onExit,
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            FutureUserAvatar(
              profile: coreClient.user
                  .userProfile(userName: widget.contentMessage.sender),
            ),
            const SizedBox(width: 20),
            Expanded(
              child: Column(
                children: [
                  SelectionContainer.disabled(
                    child: Container(
                      alignment: Alignment.topLeft,
                      padding: const EdgeInsets.only(top: 3),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          Text(
                            widget.contentMessage.sender
                                    .split("@")
                                    .firstOrNull ??
                                "",
                            style: const TextStyle(
                              color: colorDMB,
                              fontVariations: variationBold,
                              fontSize: 12,
                              letterSpacing: -0.2,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                          const SizedBox(width: 8),
                          AnimatedOpacity(
                            opacity: _hovering ? 1 : 0.0,
                            curve: Curves.easeInOut,
                            duration: const Duration(milliseconds: 200),
                            child: Text(
                              widget.contentMessage.sender.toLowerCase(),
                              style: const TextStyle(
                                color: colorDMB,
                                fontWeight: FontWeight.w400,
                                fontSize: 12,
                                letterSpacing: -0.2,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(height: 5),
                  IntrinsicHeight(
                    child: Container(
                      alignment: AlignmentDirectional.topStart,
                      padding: const EdgeInsets.only(top: 5, right: 25),
                      child: RichText(
                        text: buildTextSpanFromText(
                            ["@Alice", "@Bob", "@Carol", "@Dave", "@Eve"],
                            widget.contentMessage.content.body,
                            messageTextStyle(context),
                            HostWidget.richText),
                        selectionRegistrar: SelectionContainer.maybeOf(context),
                        selectionColor: Colors.blue.withOpacity(0.3),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            SelectionContainer.disabled(
              child: TileTimestamp(
                hovering: _hovering,
                timestamp: widget.timestamp,
              ),
            )
          ],
        ),
      ),
    );
  }
}
