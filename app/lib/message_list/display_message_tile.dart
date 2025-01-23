// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/theme/theme.dart';
import 'tile_timestamp.dart';

class DisplayMessageTile extends StatefulWidget {
  final UiEventMessage eventMessage;
  final String timestamp;
  const DisplayMessageTile(this.eventMessage, this.timestamp, {super.key});

  @override
  State<DisplayMessageTile> createState() => _DisplayMessageTileState();
}

class _DisplayMessageTileState extends State<DisplayMessageTile> {
  bool _hovering = false;

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
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Container(
            alignment: Alignment.center,
            width: 24,
            height: 24,
            child: const Icon(
              Icons.info_outline,
              color: colorDMBLight,
              size: 16,
            ),
          ),
          Expanded(
            child: Container(
              alignment: Alignment.centerLeft,
              child: switch (widget.eventMessage) {
                UiEventMessage_System(field0: final message) =>
                  SystemMessageContent(message: message),
                UiEventMessage_Error(field0: final message) =>
                  ErrorMessageContent(message: message),
              },
            ),
          ),
          TileTimestamp(hovering: _hovering, timestamp: widget.timestamp)
        ],
      ),
    );
  }
}

class SystemMessageContent extends StatelessWidget {
  const SystemMessageContent({
    super.key,
    required this.message,
  });

  final UiSystemMessage message;

  @override
  Widget build(BuildContext context) {
    return Container(
      alignment: AlignmentDirectional.centerStart,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(5.0),
        color: colorDMBSuperLight,
      ),
      padding: const EdgeInsets.all(10),
      margin: const EdgeInsets.fromLTRB(25, 0, 20, 0),
      child: Text(
        message.message,
        style: TextStyle(
          color: Colors.grey[700],
          fontVariations: variationBold,
          letterSpacing: -0.02,
          fontSize: 10,
          height: 1.4,
        ),
      ),
    );
  }
}

class ErrorMessageContent extends StatelessWidget {
  const ErrorMessageContent({
    super.key,
    required this.message,
  });

  final UiErrorMessage message;

  @override
  Widget build(BuildContext context) {
    return Container(
      alignment: AlignmentDirectional.topStart,
      child: Text(
        message.message,
        style: const TextStyle(
          color: Colors.red,
          fontWeight: FontWeight.w200,
          fontSize: 10,
          height: 1.0,
        ),
      ),
    );
  }
}
