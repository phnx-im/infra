// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:prototype/core/core.dart';
import 'package:uuid/uuid.dart';

extension IntTestExtension on int {
  ConversationId conversationId() =>
      ConversationId(uuid: _intToUuidValue(this));

  ConversationMessageId conversationMessageId() =>
      ConversationMessageId(uuid: _intToUuidValue(this));

  UiMessageId messageId({
    String domain = "localhost",
  }) =>
      UiMessageId(
        id: _intToUuidValue(this),
        domain: domain,
      );
}

UuidValue _intToUuidValue(int value) {
  // Convert int to 16-byte array
  final bytes = Uint8List(16)
    ..buffer.asByteData().setInt64(0, value, Endian.little);
  return UuidValue.fromByteList(bytes);
}
