// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:typed_data';

import 'package:prototype/core/core.dart';

extension UiConversationDetailsExtension on UiConversationDetails {
  /// Username of the conversation (for group it is the group title)
  String get username => switch (conversationType) {
        UiConversationType_UnconfirmedConnection(field0: final e) => e,
        UiConversationType_Connection(field0: final e) => e,
        UiConversationType_Group() => attributes.title,
      };

  /// Title of the conversation
  String get title => switch (conversationType) {
        UiConversationType_UnconfirmedConnection(field0: final e) => "⏳ $e",
        UiConversationType_Connection(field0: final e) => e,
        UiConversationType_Group() => attributes.title,
      };
}

extension UiConversationTypeExtension on UiConversationType {
  /// Description of the conversation type which can show in the UI
  String get description => switch (this) {
        UiConversationType_UnconfirmedConnection() =>
          "Pending connection request",
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

extension UiUserNameExtension on UiUserName {
  String displayName(String? displayName) =>
      displayName != null && displayName.isNotEmpty ? displayName : userName;
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
