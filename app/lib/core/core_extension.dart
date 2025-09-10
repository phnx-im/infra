// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:air/core/core.dart';
import 'package:air/util/platform.dart';
import 'package:uuid/uuid.dart';

extension UiConversationDetailsExtension on UiChatDetails {
  /// ClientId of the conversation (for group it is null)
  UiUserId? get userId => switch (chatType) {
    UiChatType_HandleConnection() => null,
    UiChatType_Connection(field0: final profile) => profile.userId,
    UiChatType_Group() => null,
  };

  /// Title of the conversation
  String get title => switch (chatType) {
    UiChatType_HandleConnection(field0: final handle) =>
      "⏳ ${handle.plaintext}",
    UiChatType_Connection(field0: final profile) => profile.displayName,
    UiChatType_Group() => attributes.title,
  };

  /// Picture of the conversation
  ///
  /// The picture is either the one from the conversation attributes (when this is a group
  /// conversation) or the one from the user profile (when this is a 1:1 conversation).
  ImageData? get picture => switch (chatType) {
    UiChatType_Connection(field0: final profile) => profile.profilePicture,
    UiChatType_HandleConnection() => null,
    UiChatType_Group() => attributes.picture,
  };
}

extension UiConversationTypeExtension on UiChatType {
  /// Description of the conversation type which can show in the UI
  String get description => switch (this) {
    UiChatType_HandleConnection() => "Pending connection request",
    UiChatType_Connection() => "1:1 conversation",
    UiChatType_Group() => 'Group conversation',
  };
}

extension UiFlightPositionExtension on UiFlightPosition {
  bool get isFirst => switch (this) {
    UiFlightPosition.single || UiFlightPosition.start => true,
    UiFlightPosition.middle || UiFlightPosition.end => false,
  };

  bool get isLast => switch (this) {
    UiFlightPosition.start || UiFlightPosition.middle => false,
    UiFlightPosition.single || UiFlightPosition.end => true,
  };
}

extension DeviceTokenExtension on PlatformPushToken {
  String get token => switch (this) {
    PlatformPushToken_Apple(field0: final token) => token,
    PlatformPushToken_Google(field0: final token) => token,
  };
}

extension ImageDataExtension on Uint8List {
  ImageData toImageData() =>
      ImageData(data: this, hash: ImageData.computeHash(this));
}

extension NavigationStateExtension on NavigationState {
  ChatId? get chatId => switch (this) {
    NavigationState_Home(:final home) => home.chatId,
    NavigationState_Intro() => null,
  };

  ChatId? get openChatId => switch (this) {
    NavigationState_Home(:final home) when home.chatOpen => home.chatId,
    NavigationState_Intro() || NavigationState_Home() => null,
  };
}

extension DartNotificationServiceExtension on DartNotificationService {
  static DartNotificationService create() => DartNotificationService(
    send: sendNotification,
    getActive: getActiveNotifications,
    cancel: cancelNotifications,
  );
}

extension ChatIdExtension on ChatId {
  static ChatId? fromString(String value) {
    try {
      final uuid = UuidValue.withValidation(value);
      return ChatId(uuid: uuid);
    } on FormatException catch (_) {
      return null;
    }
  }
}

extension NotificationIdExtension on NotificationId {
  static NotificationId? fromString(String value) {
    try {
      final uuid = UuidValue.withValidation(value);
      return NotificationId(field0: uuid);
    } on FormatException catch (_) {
      return null;
    }
  }
}

extension NotificationHandleExtension on NotificationHandle {
  static NotificationHandle? fromMap(Map<Object?, Object?> map) {
    final NotificationId? identifier = switch (map['identifier']) {
      String s => NotificationIdExtension.fromString(s),
      _ => null,
    };
    if (identifier == null) {
      return null;
    }
    final ChatId? chatId = switch (map['conversationId']) {
      String s => ChatIdExtension.fromString(s),
      _ => null,
    };
    return NotificationHandle(identifier: identifier, chatId: chatId);
  }
}
