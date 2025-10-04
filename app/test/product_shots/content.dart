// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:air/core/api/markdown.dart';
import 'package:air/core/core.dart';

import '../helpers.dart';

final userProfiles = [
  UiUserProfile(userId: 1.userId(), displayName: 'Alice'),
  UiUserProfile(userId: 2.userId(), displayName: 'Bob'),
  UiUserProfile(userId: 3.userId(), displayName: 'Eve'),
  UiUserProfile(userId: 4.userId(), displayName: 'Dave'),
  UiUserProfile(userId: 5.userId(), displayName: 'Frank'),
];

final chats = [
  UiChatDetails(
    id: 1.chatId(),
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(userProfiles[1]),
    unreadMessages: 10,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Bob', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 1.messageId(),
      chatId: 1.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 2.userId(),
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: 'Hey Alice! Sorry for all the spam',
            topicId: Uint8List(0),
            content: simpleMessage('Hey Alice! Sorry for all the spam'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiChatDetails(
    id: 2.chatId(),
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(userProfiles[2]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Eve', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 2.messageId(),
      chatId: 2.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          edited: true,
          content: UiMimiContent(
            plainBody: 'What is the recipe for the cake you made?',
            topicId: Uint8List(0),
            content: simpleMessage('What is the recipe for the cake you made?'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiChatDetails(
    id: 3.chatId(),
    status: const UiChatStatus.active(),
    chatType: const UiChatType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Science club', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 3.messageId(),
      chatId: 3.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 4.userId(),
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: "What is the distance to the nearest star?",
            topicId: Uint8List(0),
            content: simpleMessage("What is the distance to the nearest star?"),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiChatDetails(
    id: 4.chatId(),
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(userProfiles[3]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Dave', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 2.messageId(),
      chatId: 2.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          edited: true,
          content: UiMimiContent(
            plainBody: 'I have to tell you all about my weekend...',
            topicId: Uint8List(0),
            content: simpleMessage(
              'I have to tell you all about my weekend...',
            ),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
  UiChatDetails(
    id: 5.chatId(),
    status: const UiChatStatus.active(),
    chatType: const UiChatType_Group(),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Labubu fan club', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 3.messageId(),
      chatId: 3.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 4.userId(),
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: 'I found one for only \$800',
            topicId: Uint8List(0),
            content: simpleMessage('I found one for only \$800'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
    draft: UiMessageDraft(
      message: 'I found one for only \$800',
      editingId: null,
      updatedAt: DateTime.now(),
      source: UiMessageDraftSource.system,
    ),
  ),
  UiChatDetails(
    id: 6.chatId(),
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(userProfiles[4]),
    unreadMessages: 0,
    messagesCount: 10,
    attributes: const UiChatAttributes(title: 'Frank', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: UiChatMessage(
      id: 2.messageId(),
      chatId: 2.chatId(),
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: 3.userId(),
          sent: true,
          edited: true,
          content: UiMimiContent(
            plainBody: 'Going to the store. Need anything?',
            topicId: Uint8List(0),
            content: simpleMessage('Going to the store. Need anything?'),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    ),
  ),
];

MessageContent simpleMessage(String msg) {
  return MessageContent(
    elements: [
      RangedBlockElement(
        start: 0,
        end: msg.length,
        element: BlockElement_Paragraph([
          RangedInlineElement(
            start: 0,
            end: msg.length,
            element: InlineElement_Text(msg),
          ),
        ]),
      ),
    ],
  );
}
