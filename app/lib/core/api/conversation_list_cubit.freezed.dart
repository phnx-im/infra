// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'conversation_list_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ConversationListState {

 List<UiConversationDetails> get conversations;
/// Create a copy of ConversationListState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ConversationListStateCopyWith<ConversationListState> get copyWith => _$ConversationListStateCopyWithImpl<ConversationListState>(this as ConversationListState, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ConversationListState&&const DeepCollectionEquality().equals(other.conversations, conversations));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(conversations));

@override
String toString() {
  return 'ConversationListState(conversations: $conversations)';
}


}

/// @nodoc
abstract mixin class $ConversationListStateCopyWith<$Res>  {
  factory $ConversationListStateCopyWith(ConversationListState value, $Res Function(ConversationListState) _then) = _$ConversationListStateCopyWithImpl;
@useResult
$Res call({
 List<UiConversationDetails> conversations
});




}
/// @nodoc
class _$ConversationListStateCopyWithImpl<$Res>
    implements $ConversationListStateCopyWith<$Res> {
  _$ConversationListStateCopyWithImpl(this._self, this._then);

  final ConversationListState _self;
  final $Res Function(ConversationListState) _then;

/// Create a copy of ConversationListState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? conversations = null,}) {
  return _then(_self.copyWith(
conversations: null == conversations ? _self.conversations : conversations // ignore: cast_nullable_to_non_nullable
as List<UiConversationDetails>,
  ));
}

}


/// @nodoc


class _ConversationListState extends ConversationListState {
  const _ConversationListState({required final  List<UiConversationDetails> conversations}): _conversations = conversations,super._();
  

 final  List<UiConversationDetails> _conversations;
@override List<UiConversationDetails> get conversations {
  if (_conversations is EqualUnmodifiableListView) return _conversations;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_conversations);
}


/// Create a copy of ConversationListState
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ConversationListStateCopyWith<_ConversationListState> get copyWith => __$ConversationListStateCopyWithImpl<_ConversationListState>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ConversationListState&&const DeepCollectionEquality().equals(other._conversations, _conversations));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_conversations));

@override
String toString() {
  return 'ConversationListState(conversations: $conversations)';
}


}

/// @nodoc
abstract mixin class _$ConversationListStateCopyWith<$Res> implements $ConversationListStateCopyWith<$Res> {
  factory _$ConversationListStateCopyWith(_ConversationListState value, $Res Function(_ConversationListState) _then) = __$ConversationListStateCopyWithImpl;
@override @useResult
$Res call({
 List<UiConversationDetails> conversations
});




}
/// @nodoc
class __$ConversationListStateCopyWithImpl<$Res>
    implements _$ConversationListStateCopyWith<$Res> {
  __$ConversationListStateCopyWithImpl(this._self, this._then);

  final _ConversationListState _self;
  final $Res Function(_ConversationListState) _then;

/// Create a copy of ConversationListState
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? conversations = null,}) {
  return _then(_ConversationListState(
conversations: null == conversations ? _self._conversations : conversations // ignore: cast_nullable_to_non_nullable
as List<UiConversationDetails>,
  ));
}


}

// dart format on
