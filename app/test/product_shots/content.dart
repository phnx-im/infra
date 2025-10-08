// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:air/core/api/markdown.dart';
import 'package:air/core/core.dart';

import '../helpers.dart';

final ownId = 10.userId();

final samId = 1.userId();
final fredId = 2.userId();
final jessicaId = 3.userId();
final daveId = 4.userId();
final frankId = 5.userId();
final alexId = 6.userId();
final ireneId = 7.userId();
final kamalId = 8.userId();

final samChatId = 1.chatId();
final fredChatId = 2.chatId();
final jessicaChatId = 3.chatId();
final daveChatId = 4.chatId();
final frankChatId = 5.chatId();
final alexChatId = 6.chatId();
final ireneChatId = 7.chatId();
final kamalChatId = 8.chatId();

final scienceClubId = 10.chatId();
final gardeningClubId = 11.chatId();
final dinnerPartyId = 12.chatId();

final ownProfile = UiUserProfile(userId: ownId, displayName: 'Ellie');
final samProfile = UiUserProfile(userId: samId, displayName: 'Sam');
final fredProfile = UiUserProfile(userId: fredId, displayName: 'Bob');
final jessicaProfile = UiUserProfile(userId: jessicaId, displayName: 'Jessica');
final daveProfile = UiUserProfile(userId: daveId, displayName: 'Dave');
final frankProfile = UiUserProfile(userId: frankId, displayName: 'Frank');
final alexProfile = UiUserProfile(userId: alexId, displayName: 'Alex');
final ireneProfile = UiUserProfile(userId: ireneId, displayName: 'Irene');
final kamalProfile = UiUserProfile(userId: kamalId, displayName: 'Kamal');

final userProfiles = [
  ownProfile,
  samProfile,
  fredProfile,
  jessicaProfile,
  daveProfile,
  frankProfile,
  alexProfile,
  ireneProfile,
  kamalProfile,
];

var messageIdx = 1;

final chats = [
  // Sam
  UiChatDetails(
    id: samChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(samProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Sam', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(samChatId, samId, 'Hi! How are you?'),
  ),
  // Jessica
  UiChatDetails(
    id: jessicaChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(jessicaProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Jessica', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      jessicaChatId,
      jessicaId,
      "What is the recipe for the cake you made?",
    ),
  ),
  // Science club
  UiChatDetails(
    id: scienceClubId,
    status: const UiChatStatus.active(),
    chatType: const UiChatType_Group(),
    unreadMessages: 2,
    messagesCount: 2,
    attributes: const UiChatAttributes(title: 'Science club', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      scienceClubId,
      samId,
      "My favorite planet is Saturn. It has such cool rings. But I also like Venus a lot.",
    ),
  ),
  // Dave
  UiChatDetails(
    id: daveChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(daveProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Dave', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      daveChatId,
      daveId,
      'I have to tell you all about my weekend...',
    ),
  ),
  // Gardening club
  UiChatDetails(
    id: gardeningClubId,
    status: const UiChatStatus.active(),
    chatType: const UiChatType_Group(),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Gardening club', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      gardeningClubId,
      samId,
      "Blueberries are the best",
    ),
  ),
  // Frank
  UiChatDetails(
    id: frankChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(frankProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Frank', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      frankChatId,
      frankId,
      'Going to the store. Need anything?',
    ),
  ),
  // Alex
  UiChatDetails(
    id: alexChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(alexProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Alex', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(alexChatId, alexId, "See you there."),
  ),
  // Irene
  UiChatDetails(
    id: ireneChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(ireneProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Irene', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      ireneChatId,
      ireneId,
      "The nearest star is Proxima Centauri.",
    ),
  ),
  // Dinner party
  UiChatDetails(
    id: dinnerPartyId,
    status: const UiChatStatus.active(),
    chatType: const UiChatType_Group(),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Dinner party', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      dinnerPartyId,
      ownId,
      "Sorry, I can't join the party. I'm going to the movies already.",
    ),
  ),
  // Kamal
  UiChatDetails(
    id: kamalChatId,
    status: const UiChatStatus.active(),
    chatType: UiChatType_Connection(kamalProfile),
    unreadMessages: 0,
    messagesCount: 1,
    attributes: const UiChatAttributes(title: 'Kamal', picture: null),
    lastUsed: '2023-01-01T00:00:00.000Z',
    lastMessage: _lastChatMessage(
      kamalChatId,
      ownId,
      "Hey Kamal, I'm going to the movies with my friends. Want to come with us?",
    ),
  ),
];

UiChatMessage _lastChatMessage(ChatId chatId, UiUserId senderId, String body) =>
    UiChatMessage(
      id: (messageIdx++).messageId(),
      chatId: chatId,
      timestamp: '2023-01-01T00:00:00.000Z',
      message: UiMessage_Content(
        UiContentMessage(
          sender: senderId,
          sent: true,
          edited: false,
          content: UiMimiContent(
            plainBody: body,
            topicId: Uint8List(0),
            content: _simpleMessage(body),
            attachments: [],
          ),
        ),
      ),
      position: UiFlightPosition.single,
      status: UiMessageStatus.sent,
    );

MessageContent _simpleMessage(String msg) {
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
