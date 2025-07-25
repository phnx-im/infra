// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:prototype/core/core.dart';
import 'package:test/test.dart';

import '../conversation_list/conversation_list_content_test.dart';
import '../helpers.dart';

final bobId = 2.userId();

void main() {
  group('MessageCubit', () {
    test('UiConversationMessage equality', () {
      final a = UiConversationMessage(
        id: 1.conversationMessageId(),
        conversationId: 1.conversationId(),
        timestamp: '2023-01-01T00:00:00.000Z',
        message: UiMessage_Content(
          UiContentMessage(
            sender: bobId,
            sent: true,
            edited: false,
            content: UiMimiContent(
              plainBody: 'Hello Alice',
              topicId: Uint8List(0),
              content: simpleMessage('Hello Alice'),
              attachments: [],
            ),
          ),
        ),
        position: UiFlightPosition.single,
        status: UiMessageStatus.sent,
      );
      final b = UiConversationMessage(
        id: 1.conversationMessageId(),
        conversationId: 1.conversationId(),
        timestamp: '2023-01-01T00:00:00.000Z',
        message: UiMessage_Content(
          UiContentMessage(
            sender: bobId,
            sent: true,
            edited: false,
            content: UiMimiContent(
              plainBody: 'Hello Alice',
              topicId: Uint8List(0),
              content: simpleMessage('Hello Alice'),
              attachments: [],
            ),
          ),
        ),
        position: UiFlightPosition.single,
        status: UiMessageStatus.sent,
      );
      expect(a, equals(b));
    });
  });
}
