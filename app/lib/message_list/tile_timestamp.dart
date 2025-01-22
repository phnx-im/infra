// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/styles.dart';

class TileTimestamp extends StatelessWidget {
  const TileTimestamp({
    super.key,
    required bool hovering,
    required this.timestamp,
  }) : _hovering = hovering;

  final bool _hovering;
  final String timestamp;

  @override
  Widget build(BuildContext context) {
    return AnimatedOpacity(
      opacity: _hovering ? 1 : 0,
      curve: Curves.easeInOut,
      duration: const Duration(milliseconds: 300),
      child: Container(
        alignment: AlignmentDirectional.topEnd,
        padding: const EdgeInsets.only(top: 5),
        width: 36,
        child: Text(
          timeString(timestamp),
          style: const TextStyle(
            color: colorDMB,
            fontWeight: FontWeight.w200,
            fontSize: 10,
            letterSpacing: -0.2,
          ),
        ),
      ),
    );
  }
}

String timeString(String timestamp) {
  final t = DateTime.parse(timestamp);
  return '${t.hour}:${t.minute.toString().padLeft(2, '0')}';
}
