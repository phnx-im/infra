// This file is automatically generated, so please do not edit it.
// Generated by `flutter_rust_bridge`@ 2.0.0-dev.36.

// ignore_for_file: unused_import, unused_element, unnecessary_import, duplicate_ignore, invalid_use_of_internal_member, annotate_overrides, non_constant_identifier_names, curly_braces_in_flow_control_structures, prefer_const_literals_to_create_immutables, unused_field

import 'dart:async';
import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart_api.dart';
import 'frb_generated.dart';
import 'mobile_logging.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart';
import 'types.dart';

abstract class RustLibApiImplPlatform extends BaseApiImpl<RustLibWire> {
  RustLibApiImplPlatform({
    required super.handler,
    required super.wire,
    required super.generalizedFrbRustBinding,
    required super.portManager,
  });

  CrossPlatformFinalizerArg get rust_arc_decrement_strong_count_RustUserPtr => wire
      ._rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUserPtr;

  CrossPlatformFinalizerArg
      get rust_arc_decrement_strong_count_UserBuilderPtr => wire
          ._rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilderPtr;

  @protected
  AnyhowException dco_decode_AnyhowException(dynamic raw);

  @protected
  RustUser
      dco_decode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          dynamic raw);

  @protected
  UserBuilder
      dco_decode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          dynamic raw);

  @protected
  RustUser
      dco_decode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          dynamic raw);

  @protected
  UserBuilder
      dco_decode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          dynamic raw);

  @protected
  DateTime dco_decode_Chrono_Utc(dynamic raw);

  @protected
  RustUser
      dco_decode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          dynamic raw);

  @protected
  UserBuilder
      dco_decode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          dynamic raw);

  @protected
  RustStreamSink<LogEntry> dco_decode_StreamSink_log_entry_Sse(dynamic raw);

  @protected
  RustStreamSink<UiNotificationType>
      dco_decode_StreamSink_ui_notification_type_Sse(dynamic raw);

  @protected
  RustStreamSink<WsNotification> dco_decode_StreamSink_ws_notification_Sse(
      dynamic raw);

  @protected
  String dco_decode_String(dynamic raw);

  @protected
  bool dco_decode_bool(dynamic raw);

  @protected
  ConversationIdBytes dco_decode_box_autoadd_conversation_id_bytes(dynamic raw);

  @protected
  BigInt dco_decode_box_autoadd_u_64(dynamic raw);

  @protected
  UiContact dco_decode_box_autoadd_ui_contact(dynamic raw);

  @protected
  UiContentMessage dco_decode_box_autoadd_ui_content_message(dynamic raw);

  @protected
  UiConversation dco_decode_box_autoadd_ui_conversation(dynamic raw);

  @protected
  UiConversationMessage dco_decode_box_autoadd_ui_conversation_message(
      dynamic raw);

  @protected
  UiErrorMessage dco_decode_box_autoadd_ui_error_message(dynamic raw);

  @protected
  UiEventMessage dco_decode_box_autoadd_ui_event_message(dynamic raw);

  @protected
  UiInactiveConversation dco_decode_box_autoadd_ui_inactive_conversation(
      dynamic raw);

  @protected
  UiMessageId dco_decode_box_autoadd_ui_message_id(dynamic raw);

  @protected
  UiMimiContent dco_decode_box_autoadd_ui_mimi_content(dynamic raw);

  @protected
  UiNotificationType dco_decode_box_autoadd_ui_notification_type(dynamic raw);

  @protected
  UiReplyToInfo dco_decode_box_autoadd_ui_reply_to_info(dynamic raw);

  @protected
  UiSystemMessage dco_decode_box_autoadd_ui_system_message(dynamic raw);

  @protected
  UiUserProfile dco_decode_box_autoadd_ui_user_profile(dynamic raw);

  @protected
  ConversationIdBytes dco_decode_conversation_id_bytes(dynamic raw);

  @protected
  GroupIdBytes dco_decode_group_id_bytes(dynamic raw);

  @protected
  int dco_decode_i_32(dynamic raw);

  @protected
  PlatformInt64 dco_decode_i_64(dynamic raw);

  @protected
  List<String> dco_decode_list_String(dynamic raw);

  @protected
  Uint8List dco_decode_list_prim_u_8_strict(dynamic raw);

  @protected
  List<UiContact> dco_decode_list_ui_contact(dynamic raw);

  @protected
  List<UiConversation> dco_decode_list_ui_conversation(dynamic raw);

  @protected
  List<UiConversationMessage> dco_decode_list_ui_conversation_message(
      dynamic raw);

  @protected
  List<UiMessageId> dco_decode_list_ui_message_id(dynamic raw);

  @protected
  LogEntry dco_decode_log_entry(dynamic raw);

  @protected
  String? dco_decode_opt_String(dynamic raw);

  @protected
  BigInt? dco_decode_opt_box_autoadd_u_64(dynamic raw);

  @protected
  UiContact? dco_decode_opt_box_autoadd_ui_contact(dynamic raw);

  @protected
  UiMessageId? dco_decode_opt_box_autoadd_ui_message_id(dynamic raw);

  @protected
  UiReplyToInfo? dco_decode_opt_box_autoadd_ui_reply_to_info(dynamic raw);

  @protected
  UiUserProfile? dco_decode_opt_box_autoadd_ui_user_profile(dynamic raw);

  @protected
  Uint8List? dco_decode_opt_list_prim_u_8_strict(dynamic raw);

  @protected
  int dco_decode_u_32(dynamic raw);

  @protected
  BigInt dco_decode_u_64(dynamic raw);

  @protected
  int dco_decode_u_8(dynamic raw);

  @protected
  U8Array16 dco_decode_u_8_array_16(dynamic raw);

  @protected
  UiContact dco_decode_ui_contact(dynamic raw);

  @protected
  UiContentMessage dco_decode_ui_content_message(dynamic raw);

  @protected
  UiConversation dco_decode_ui_conversation(dynamic raw);

  @protected
  UiConversationAttributes dco_decode_ui_conversation_attributes(dynamic raw);

  @protected
  UiConversationMessage dco_decode_ui_conversation_message(dynamic raw);

  @protected
  UiConversationStatus dco_decode_ui_conversation_status(dynamic raw);

  @protected
  UiConversationType dco_decode_ui_conversation_type(dynamic raw);

  @protected
  UiErrorMessage dco_decode_ui_error_message(dynamic raw);

  @protected
  UiEventMessage dco_decode_ui_event_message(dynamic raw);

  @protected
  UiInactiveConversation dco_decode_ui_inactive_conversation(dynamic raw);

  @protected
  UiMessage dco_decode_ui_message(dynamic raw);

  @protected
  UiMessageId dco_decode_ui_message_id(dynamic raw);

  @protected
  UiMimiContent dco_decode_ui_mimi_content(dynamic raw);

  @protected
  UiNotificationType dco_decode_ui_notification_type(dynamic raw);

  @protected
  UiReplyToInfo dco_decode_ui_reply_to_info(dynamic raw);

  @protected
  UiSystemMessage dco_decode_ui_system_message(dynamic raw);

  @protected
  UiUserProfile dco_decode_ui_user_profile(dynamic raw);

  @protected
  void dco_decode_unit(dynamic raw);

  @protected
  BigInt dco_decode_usize(dynamic raw);

  @protected
  UuidBytes dco_decode_uuid_bytes(dynamic raw);

  @protected
  WsNotification dco_decode_ws_notification(dynamic raw);

  @protected
  AnyhowException sse_decode_AnyhowException(SseDeserializer deserializer);

  @protected
  RustUser
      sse_decode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          SseDeserializer deserializer);

  @protected
  UserBuilder
      sse_decode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          SseDeserializer deserializer);

  @protected
  RustUser
      sse_decode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          SseDeserializer deserializer);

  @protected
  UserBuilder
      sse_decode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          SseDeserializer deserializer);

  @protected
  DateTime sse_decode_Chrono_Utc(SseDeserializer deserializer);

  @protected
  RustUser
      sse_decode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          SseDeserializer deserializer);

  @protected
  UserBuilder
      sse_decode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          SseDeserializer deserializer);

  @protected
  RustStreamSink<LogEntry> sse_decode_StreamSink_log_entry_Sse(
      SseDeserializer deserializer);

  @protected
  RustStreamSink<UiNotificationType>
      sse_decode_StreamSink_ui_notification_type_Sse(
          SseDeserializer deserializer);

  @protected
  RustStreamSink<WsNotification> sse_decode_StreamSink_ws_notification_Sse(
      SseDeserializer deserializer);

  @protected
  String sse_decode_String(SseDeserializer deserializer);

  @protected
  bool sse_decode_bool(SseDeserializer deserializer);

  @protected
  ConversationIdBytes sse_decode_box_autoadd_conversation_id_bytes(
      SseDeserializer deserializer);

  @protected
  BigInt sse_decode_box_autoadd_u_64(SseDeserializer deserializer);

  @protected
  UiContact sse_decode_box_autoadd_ui_contact(SseDeserializer deserializer);

  @protected
  UiContentMessage sse_decode_box_autoadd_ui_content_message(
      SseDeserializer deserializer);

  @protected
  UiConversation sse_decode_box_autoadd_ui_conversation(
      SseDeserializer deserializer);

  @protected
  UiConversationMessage sse_decode_box_autoadd_ui_conversation_message(
      SseDeserializer deserializer);

  @protected
  UiErrorMessage sse_decode_box_autoadd_ui_error_message(
      SseDeserializer deserializer);

  @protected
  UiEventMessage sse_decode_box_autoadd_ui_event_message(
      SseDeserializer deserializer);

  @protected
  UiInactiveConversation sse_decode_box_autoadd_ui_inactive_conversation(
      SseDeserializer deserializer);

  @protected
  UiMessageId sse_decode_box_autoadd_ui_message_id(
      SseDeserializer deserializer);

  @protected
  UiMimiContent sse_decode_box_autoadd_ui_mimi_content(
      SseDeserializer deserializer);

  @protected
  UiNotificationType sse_decode_box_autoadd_ui_notification_type(
      SseDeserializer deserializer);

  @protected
  UiReplyToInfo sse_decode_box_autoadd_ui_reply_to_info(
      SseDeserializer deserializer);

  @protected
  UiSystemMessage sse_decode_box_autoadd_ui_system_message(
      SseDeserializer deserializer);

  @protected
  UiUserProfile sse_decode_box_autoadd_ui_user_profile(
      SseDeserializer deserializer);

  @protected
  ConversationIdBytes sse_decode_conversation_id_bytes(
      SseDeserializer deserializer);

  @protected
  GroupIdBytes sse_decode_group_id_bytes(SseDeserializer deserializer);

  @protected
  int sse_decode_i_32(SseDeserializer deserializer);

  @protected
  PlatformInt64 sse_decode_i_64(SseDeserializer deserializer);

  @protected
  List<String> sse_decode_list_String(SseDeserializer deserializer);

  @protected
  Uint8List sse_decode_list_prim_u_8_strict(SseDeserializer deserializer);

  @protected
  List<UiContact> sse_decode_list_ui_contact(SseDeserializer deserializer);

  @protected
  List<UiConversation> sse_decode_list_ui_conversation(
      SseDeserializer deserializer);

  @protected
  List<UiConversationMessage> sse_decode_list_ui_conversation_message(
      SseDeserializer deserializer);

  @protected
  List<UiMessageId> sse_decode_list_ui_message_id(SseDeserializer deserializer);

  @protected
  LogEntry sse_decode_log_entry(SseDeserializer deserializer);

  @protected
  String? sse_decode_opt_String(SseDeserializer deserializer);

  @protected
  BigInt? sse_decode_opt_box_autoadd_u_64(SseDeserializer deserializer);

  @protected
  UiContact? sse_decode_opt_box_autoadd_ui_contact(
      SseDeserializer deserializer);

  @protected
  UiMessageId? sse_decode_opt_box_autoadd_ui_message_id(
      SseDeserializer deserializer);

  @protected
  UiReplyToInfo? sse_decode_opt_box_autoadd_ui_reply_to_info(
      SseDeserializer deserializer);

  @protected
  UiUserProfile? sse_decode_opt_box_autoadd_ui_user_profile(
      SseDeserializer deserializer);

  @protected
  Uint8List? sse_decode_opt_list_prim_u_8_strict(SseDeserializer deserializer);

  @protected
  int sse_decode_u_32(SseDeserializer deserializer);

  @protected
  BigInt sse_decode_u_64(SseDeserializer deserializer);

  @protected
  int sse_decode_u_8(SseDeserializer deserializer);

  @protected
  U8Array16 sse_decode_u_8_array_16(SseDeserializer deserializer);

  @protected
  UiContact sse_decode_ui_contact(SseDeserializer deserializer);

  @protected
  UiContentMessage sse_decode_ui_content_message(SseDeserializer deserializer);

  @protected
  UiConversation sse_decode_ui_conversation(SseDeserializer deserializer);

  @protected
  UiConversationAttributes sse_decode_ui_conversation_attributes(
      SseDeserializer deserializer);

  @protected
  UiConversationMessage sse_decode_ui_conversation_message(
      SseDeserializer deserializer);

  @protected
  UiConversationStatus sse_decode_ui_conversation_status(
      SseDeserializer deserializer);

  @protected
  UiConversationType sse_decode_ui_conversation_type(
      SseDeserializer deserializer);

  @protected
  UiErrorMessage sse_decode_ui_error_message(SseDeserializer deserializer);

  @protected
  UiEventMessage sse_decode_ui_event_message(SseDeserializer deserializer);

  @protected
  UiInactiveConversation sse_decode_ui_inactive_conversation(
      SseDeserializer deserializer);

  @protected
  UiMessage sse_decode_ui_message(SseDeserializer deserializer);

  @protected
  UiMessageId sse_decode_ui_message_id(SseDeserializer deserializer);

  @protected
  UiMimiContent sse_decode_ui_mimi_content(SseDeserializer deserializer);

  @protected
  UiNotificationType sse_decode_ui_notification_type(
      SseDeserializer deserializer);

  @protected
  UiReplyToInfo sse_decode_ui_reply_to_info(SseDeserializer deserializer);

  @protected
  UiSystemMessage sse_decode_ui_system_message(SseDeserializer deserializer);

  @protected
  UiUserProfile sse_decode_ui_user_profile(SseDeserializer deserializer);

  @protected
  void sse_decode_unit(SseDeserializer deserializer);

  @protected
  BigInt sse_decode_usize(SseDeserializer deserializer);

  @protected
  UuidBytes sse_decode_uuid_bytes(SseDeserializer deserializer);

  @protected
  WsNotification sse_decode_ws_notification(SseDeserializer deserializer);

  @protected
  void sse_encode_AnyhowException(
      AnyhowException self, SseSerializer serializer);

  @protected
  void
      sse_encode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          RustUser self, SseSerializer serializer);

  @protected
  void
      sse_encode_Auto_Owned_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          UserBuilder self, SseSerializer serializer);

  @protected
  void
      sse_encode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          RustUser self, SseSerializer serializer);

  @protected
  void
      sse_encode_Auto_Ref_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          UserBuilder self, SseSerializer serializer);

  @protected
  void sse_encode_Chrono_Utc(DateTime self, SseSerializer serializer);

  @protected
  void
      sse_encode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
          RustUser self, SseSerializer serializer);

  @protected
  void
      sse_encode_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
          UserBuilder self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_log_entry_Sse(
      RustStreamSink<LogEntry> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_ui_notification_type_Sse(
      RustStreamSink<UiNotificationType> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_ws_notification_Sse(
      RustStreamSink<WsNotification> self, SseSerializer serializer);

  @protected
  void sse_encode_String(String self, SseSerializer serializer);

  @protected
  void sse_encode_bool(bool self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_conversation_id_bytes(
      ConversationIdBytes self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_u_64(BigInt self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_contact(
      UiContact self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_content_message(
      UiContentMessage self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_conversation(
      UiConversation self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_conversation_message(
      UiConversationMessage self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_error_message(
      UiErrorMessage self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_event_message(
      UiEventMessage self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_inactive_conversation(
      UiInactiveConversation self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_message_id(
      UiMessageId self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_mimi_content(
      UiMimiContent self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_notification_type(
      UiNotificationType self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_reply_to_info(
      UiReplyToInfo self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_system_message(
      UiSystemMessage self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_ui_user_profile(
      UiUserProfile self, SseSerializer serializer);

  @protected
  void sse_encode_conversation_id_bytes(
      ConversationIdBytes self, SseSerializer serializer);

  @protected
  void sse_encode_group_id_bytes(GroupIdBytes self, SseSerializer serializer);

  @protected
  void sse_encode_i_32(int self, SseSerializer serializer);

  @protected
  void sse_encode_i_64(PlatformInt64 self, SseSerializer serializer);

  @protected
  void sse_encode_list_String(List<String> self, SseSerializer serializer);

  @protected
  void sse_encode_list_prim_u_8_strict(
      Uint8List self, SseSerializer serializer);

  @protected
  void sse_encode_list_ui_contact(
      List<UiContact> self, SseSerializer serializer);

  @protected
  void sse_encode_list_ui_conversation(
      List<UiConversation> self, SseSerializer serializer);

  @protected
  void sse_encode_list_ui_conversation_message(
      List<UiConversationMessage> self, SseSerializer serializer);

  @protected
  void sse_encode_list_ui_message_id(
      List<UiMessageId> self, SseSerializer serializer);

  @protected
  void sse_encode_log_entry(LogEntry self, SseSerializer serializer);

  @protected
  void sse_encode_opt_String(String? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_u_64(BigInt? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_ui_contact(
      UiContact? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_ui_message_id(
      UiMessageId? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_ui_reply_to_info(
      UiReplyToInfo? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_ui_user_profile(
      UiUserProfile? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_list_prim_u_8_strict(
      Uint8List? self, SseSerializer serializer);

  @protected
  void sse_encode_u_32(int self, SseSerializer serializer);

  @protected
  void sse_encode_u_64(BigInt self, SseSerializer serializer);

  @protected
  void sse_encode_u_8(int self, SseSerializer serializer);

  @protected
  void sse_encode_u_8_array_16(U8Array16 self, SseSerializer serializer);

  @protected
  void sse_encode_ui_contact(UiContact self, SseSerializer serializer);

  @protected
  void sse_encode_ui_content_message(
      UiContentMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_conversation(
      UiConversation self, SseSerializer serializer);

  @protected
  void sse_encode_ui_conversation_attributes(
      UiConversationAttributes self, SseSerializer serializer);

  @protected
  void sse_encode_ui_conversation_message(
      UiConversationMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_conversation_status(
      UiConversationStatus self, SseSerializer serializer);

  @protected
  void sse_encode_ui_conversation_type(
      UiConversationType self, SseSerializer serializer);

  @protected
  void sse_encode_ui_error_message(
      UiErrorMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_event_message(
      UiEventMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_inactive_conversation(
      UiInactiveConversation self, SseSerializer serializer);

  @protected
  void sse_encode_ui_message(UiMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_message_id(UiMessageId self, SseSerializer serializer);

  @protected
  void sse_encode_ui_mimi_content(UiMimiContent self, SseSerializer serializer);

  @protected
  void sse_encode_ui_notification_type(
      UiNotificationType self, SseSerializer serializer);

  @protected
  void sse_encode_ui_reply_to_info(
      UiReplyToInfo self, SseSerializer serializer);

  @protected
  void sse_encode_ui_system_message(
      UiSystemMessage self, SseSerializer serializer);

  @protected
  void sse_encode_ui_user_profile(UiUserProfile self, SseSerializer serializer);

  @protected
  void sse_encode_unit(void self, SseSerializer serializer);

  @protected
  void sse_encode_usize(BigInt self, SseSerializer serializer);

  @protected
  void sse_encode_uuid_bytes(UuidBytes self, SseSerializer serializer);

  @protected
  void sse_encode_ws_notification(
      WsNotification self, SseSerializer serializer);
}

// Section: wire_class

class RustLibWire implements BaseWire {
  factory RustLibWire.fromExternalLibrary(ExternalLibrary lib) =>
      RustLibWire(lib.ffiDynamicLibrary);

  /// Holds the symbol lookup function.
  final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
      _lookup;

  /// The symbols are looked up in [dynamicLibrary].
  RustLibWire(ffi.DynamicLibrary dynamicLibrary)
      : _lookup = dynamicLibrary.lookup;

  void
      rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
    ffi.Pointer<ffi.Void> ptr,
  ) {
    return _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
      ptr,
    );
  }

  late final _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUserPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Void>)>>(
          'frbgen_prototype_rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser');
  late final _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser =
      _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUserPtr
          .asFunction<void Function(ffi.Pointer<ffi.Void>)>();

  void
      rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
    ffi.Pointer<ffi.Void> ptr,
  ) {
    return _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser(
      ptr,
    );
  }

  late final _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUserPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Void>)>>(
          'frbgen_prototype_rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser');
  late final _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUser =
      _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerRustUserPtr
          .asFunction<void Function(ffi.Pointer<ffi.Void>)>();

  void
      rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
    ffi.Pointer<ffi.Void> ptr,
  ) {
    return _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
      ptr,
    );
  }

  late final _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilderPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Void>)>>(
          'frbgen_prototype_rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder');
  late final _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder =
      _rust_arc_increment_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilderPtr
          .asFunction<void Function(ffi.Pointer<ffi.Void>)>();

  void
      rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
    ffi.Pointer<ffi.Void> ptr,
  ) {
    return _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder(
      ptr,
    );
  }

  late final _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilderPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Void>)>>(
          'frbgen_prototype_rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder');
  late final _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilder =
      _rust_arc_decrement_strong_count_RustOpaque_flutter_rust_bridgefor_generatedRustAutoOpaqueInnerUserBuilderPtr
          .asFunction<void Function(ffi.Pointer<ffi.Void>)>();
}
