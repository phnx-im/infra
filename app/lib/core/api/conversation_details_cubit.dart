// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.7.1.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:freezed_annotation/freezed_annotation.dart' hide protected;
import 'package:uuid/uuid.dart';
import 'types.dart';
import 'user_cubit.dart';
part 'conversation_details_cubit.freezed.dart';

// These functions are ignored because they are not marked as `pub`: `handle_store_notification`, `load_and_emit_state`, `load_conversation_details`, `members_of_conversation`, `new`, `spawn`, `store_notifications_loop`
// These types are ignored because they are not used by any `pub` functions: `ConversationDetailsContext`, `MarkAsReadState`
// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `assert_receiver_is_total_eq`, `clone`, `clone`, `eq`, `fmt`, `fmt`, `hash`
// These functions are ignored (category: IgnoreBecauseOwnerTyShouldIgnore): `default`

// Rust type: RustOpaqueMoi<flutter_rust_bridge::for_generated::RustAutoOpaqueInner<ConversationDetailsCubitBase>>
abstract class ConversationDetailsCubitBase implements RustOpaqueInterface {
  Future<void> close();

  bool get isClosed;

  /// Load user profile of the conversation (only for non-group conversations)
  Future<UiUserProfile?> loadConversationUserProfile();

  /// Marks the conversation as read until the given message id (including).
  ///
  /// The calls to this method are debounced with a fixed delay.
  Future<void> markAsRead(
      {required ConversationMessageId untilMessageId,
      required DateTime untilTimestamp});

  /// Creates a new cubit for the given conversation.
  ///
  /// The cubit will fetch the conversation details and the list of members. It will also listen
  /// to the changes in the conversation and update the state accordingly.
  factory ConversationDetailsCubitBase(
          {required UserCubitBase userCubit,
          required ConversationId conversationId}) =>
      RustLib.instance.api
          .crateApiConversationDetailsCubitConversationDetailsCubitBaseNew(
              userCubit: userCubit, conversationId: conversationId);

  /// Sends a message to the conversation.
  ///
  /// The not yet sent message is immediately stored in the local store and then the message is
  /// send to the DS.
  Future<void> sendMessage({required String messageText});

  /// Sets the conversation picture.
  ///
  /// When `bytes` is `None`, the conversation picture is removed.
  Future<void> setConversationPicture({Uint8List? bytes});

  ConversationDetailsState get state;

  Stream<ConversationDetailsState> stream();
}

/// The state of a single conversation
///
/// Contains the conversation details and the list of members.
///
/// Also see [`ConversationDetailsCubitBase`].
@freezed
class ConversationDetailsState with _$ConversationDetailsState {
  const ConversationDetailsState._();
  const factory ConversationDetailsState({
    UiConversationDetails? conversation,
    required List<String> members,
  }) = _ConversationDetailsState;
  static Future<ConversationDetailsState> default_() => RustLib.instance.api
      .crateApiConversationDetailsCubitConversationDetailsStateDefault();
}
