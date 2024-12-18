// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:prototype/core/api/types.dart';

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

  String get avatarCacheTag =>
      'conv:$id:${attributes.conversationPictureOption?.hashCode}';
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
