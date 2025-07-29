// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'conversation_details_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ConversationDetailsState {

 UiConversationDetails? get conversation; List<UiUserId> get members; UiRoomState? get roomState;
/// Create a copy of ConversationDetailsState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ConversationDetailsStateCopyWith<ConversationDetailsState> get copyWith => _$ConversationDetailsStateCopyWithImpl<ConversationDetailsState>(this as ConversationDetailsState, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConversationDetailsState&&(identical(other.conversation, conversation) || other.conversation == conversation)&&const DeepCollectionEquality().equals(other.members, members)&&(identical(other.roomState, roomState) || other.roomState == roomState));
}


@override
int get hashCode => Object.hash(runtimeType,conversation,const DeepCollectionEquality().hash(members),roomState);

@override
String toString() {
  return 'ConversationDetailsState(conversation: $conversation, members: $members, roomState: $roomState)';
}


}

/// @nodoc
abstract mixin class $ConversationDetailsStateCopyWith<$Res>  {
  factory $ConversationDetailsStateCopyWith(ConversationDetailsState value, $Res Function(ConversationDetailsState) _then) = _$ConversationDetailsStateCopyWithImpl;
@useResult
$Res call({
 UiConversationDetails? conversation, List<UiUserId> members, UiRoomState? roomState
});




}
/// @nodoc
class _$ConversationDetailsStateCopyWithImpl<$Res>
    implements $ConversationDetailsStateCopyWith<$Res> {
  _$ConversationDetailsStateCopyWithImpl(this._self, this._then);

  final ConversationDetailsState _self;
  final $Res Function(ConversationDetailsState) _then;

/// Create a copy of ConversationDetailsState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? conversation = freezed,Object? members = null,Object? roomState = freezed,}) {
  return _then(_self.copyWith(
conversation: freezed == conversation ? _self.conversation : conversation // ignore: cast_nullable_to_non_nullable
as UiConversationDetails?,members: null == members ? _self.members : members // ignore: cast_nullable_to_non_nullable
as List<UiUserId>,roomState: freezed == roomState ? _self.roomState : roomState // ignore: cast_nullable_to_non_nullable
as UiRoomState?,
  ));
}

}


/// @nodoc


class _ConversationDetailsState extends ConversationDetailsState {
  const _ConversationDetailsState({this.conversation, required final  List<UiUserId> members, this.roomState}): _members = members,super._();
  

@override final  UiConversationDetails? conversation;
 final  List<UiUserId> _members;
@override List<UiUserId> get members {
  if (_members is EqualUnmodifiableListView) return _members;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_members);
}

@override final  UiRoomState? roomState;

/// Create a copy of ConversationDetailsState
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ConversationDetailsStateCopyWith<_ConversationDetailsState> get copyWith => __$ConversationDetailsStateCopyWithImpl<_ConversationDetailsState>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ConversationDetailsState&&(identical(other.conversation, conversation) || other.conversation == conversation)&&const DeepCollectionEquality().equals(other._members, _members)&&(identical(other.roomState, roomState) || other.roomState == roomState));
}


@override
int get hashCode => Object.hash(runtimeType,conversation,const DeepCollectionEquality().hash(_members),roomState);

@override
String toString() {
  return 'ConversationDetailsState(conversation: $conversation, members: $members, roomState: $roomState)';
}


}

/// @nodoc
abstract mixin class _$ConversationDetailsStateCopyWith<$Res> implements $ConversationDetailsStateCopyWith<$Res> {
  factory _$ConversationDetailsStateCopyWith(_ConversationDetailsState value, $Res Function(_ConversationDetailsState) _then) = __$ConversationDetailsStateCopyWithImpl;
@override @useResult
$Res call({
 UiConversationDetails? conversation, List<UiUserId> members, UiRoomState? roomState
});




}
/// @nodoc
class __$ConversationDetailsStateCopyWithImpl<$Res>
    implements _$ConversationDetailsStateCopyWith<$Res> {
  __$ConversationDetailsStateCopyWithImpl(this._self, this._then);

  final _ConversationDetailsState _self;
  final $Res Function(_ConversationDetailsState) _then;

/// Create a copy of ConversationDetailsState
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? conversation = freezed,Object? members = null,Object? roomState = freezed,}) {
  return _then(_ConversationDetailsState(
conversation: freezed == conversation ? _self.conversation : conversation // ignore: cast_nullable_to_non_nullable
as UiConversationDetails?,members: null == members ? _self._members : members // ignore: cast_nullable_to_non_nullable
as List<UiUserId>,roomState: freezed == roomState ? _self.roomState : roomState // ignore: cast_nullable_to_non_nullable
as UiRoomState?,
  ));
}


}

// dart format on
