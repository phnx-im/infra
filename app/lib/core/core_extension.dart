// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:prototype/core/core.dart';
import 'package:prototype/util/platform.dart';
import 'package:uuid/uuid.dart';

extension UiConversationDetailsExtension on UiConversationDetails {
  /// ClientId of the conversation (for group it is null)
  UiUserId? get userId => switch (conversationType) {
    UiConversationType_UnconfirmedConnection(field0: final profile) =>
      profile.userId,
    UiConversationType_HandleConnection() => null,
    UiConversationType_Connection(field0: final profile) => profile.userId,
    UiConversationType_Group() => null,
  };

  /// Title of the conversation
  String get title => switch (conversationType) {
    UiConversationType_UnconfirmedConnection(field0: final profile) =>
      "⏳ ${profile.displayName}",
    UiConversationType_HandleConnection(field0: final handle) =>
      "⏳ ${handle.plaintext}",
    UiConversationType_Connection(field0: final profile) => profile.displayName,
    UiConversationType_Group() => attributes.title,
  };

  /// Picture of the conversation
  ///
  /// The picture is either the one from the conversation attributes (when this is a group
  /// conversation) or the one from the user profile (when this is a 1:1 conversation).
  ImageData? get picture => switch (conversationType) {
    UiConversationType_Connection(field0: final profile) ||
    UiConversationType_UnconfirmedConnection(
      field0: final profile,
    ) => profile.profilePicture,
    UiConversationType_HandleConnection() => null,
    UiConversationType_Group() => attributes.picture,
  };
}

extension UiConversationTypeExtension on UiConversationType {
  /// Description of the conversation type which can show in the UI
  String get description => switch (this) {
    UiConversationType_UnconfirmedConnection() ||
    UiConversationType_HandleConnection() => "Pending connection request",
    UiConversationType_Connection() => "1:1 conversation",
    UiConversationType_Group() => 'Group conversation',
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
  ConversationId? get conversationId => switch (this) {
    NavigationState_Home(:final home) => home.conversationId,
    NavigationState_Intro() => null,
  };

  ConversationId? get openConversationId => switch (this) {
    NavigationState_Home(:final home) when home.conversationOpen =>
      home.conversationId,
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

extension ConversationIdExtension on ConversationId {
  static ConversationId? fromString(String value) {
    try {
      final uuid = UuidValue.withValidation(value);
      return ConversationId(uuid: uuid);
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
    final ConversationId? conversationId = switch (map['conversationId']) {
      String s => ConversationIdExtension.fromString(s),
      _ => null,
    };
    return NotificationHandle(
      identifier: identifier,
      conversationId: conversationId,
    );
  }
}
