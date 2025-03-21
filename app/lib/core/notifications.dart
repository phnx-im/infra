// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.9.0.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import 'api/types.dart';
import 'frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:uuid/uuid.dart';

class NotificationContent {
  final NotificationId identifier;
  final String title;
  final String body;
  final ConversationId? conversationId;

  const NotificationContent({
    required this.identifier,
    required this.title,
    required this.body,
    this.conversationId,
  });

  @override
  int get hashCode =>
      identifier.hashCode ^
      title.hashCode ^
      body.hashCode ^
      conversationId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationContent &&
          runtimeType == other.runtimeType &&
          identifier == other.identifier &&
          title == other.title &&
          body == other.body &&
          conversationId == other.conversationId;
}

class NotificationHandle {
  final NotificationId identifier;
  final ConversationId? conversationId;

  const NotificationHandle({
    required this.identifier,
    this.conversationId,
  });

  @override
  int get hashCode => identifier.hashCode ^ conversationId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationHandle &&
          runtimeType == other.runtimeType &&
          identifier == other.identifier &&
          conversationId == other.conversationId;
}

class NotificationId {
  final UuidValue field0;

  const NotificationId({
    required this.field0,
  });

  @override
  int get hashCode => field0.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationId &&
          runtimeType == other.runtimeType &&
          field0 == other.field0;
}
