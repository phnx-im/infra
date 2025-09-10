// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiChatStatus {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatStatus);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiChatStatus()';
}


}

/// @nodoc
class $UiChatStatusCopyWith<$Res>  {
$UiChatStatusCopyWith(UiChatStatus _, $Res Function(UiChatStatus) __);
}



/// @nodoc


class UiChatStatus_Inactive extends UiChatStatus {
  const UiChatStatus_Inactive(this.field0): super._();
  

 final  UiInactiveChat field0;

/// Create a copy of UiChatStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiChatStatus_InactiveCopyWith<UiChatStatus_Inactive> get copyWith => _$UiChatStatus_InactiveCopyWithImpl<UiChatStatus_Inactive>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatStatus_Inactive&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiChatStatus.inactive(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiChatStatus_InactiveCopyWith<$Res> implements $UiChatStatusCopyWith<$Res> {
  factory $UiChatStatus_InactiveCopyWith(UiChatStatus_Inactive value, $Res Function(UiChatStatus_Inactive) _then) = _$UiChatStatus_InactiveCopyWithImpl;
@useResult
$Res call({
 UiInactiveChat field0
});




}
/// @nodoc
class _$UiChatStatus_InactiveCopyWithImpl<$Res>
    implements $UiChatStatus_InactiveCopyWith<$Res> {
  _$UiChatStatus_InactiveCopyWithImpl(this._self, this._then);

  final UiChatStatus_Inactive _self;
  final $Res Function(UiChatStatus_Inactive) _then;

/// Create a copy of UiChatStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiChatStatus_Inactive(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiInactiveChat,
  ));
}


}

/// @nodoc


class UiChatStatus_Active extends UiChatStatus {
  const UiChatStatus_Active(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatStatus_Active);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiChatStatus.active()';
}


}




/// @nodoc
mixin _$UiChatType {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatType);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiChatType()';
}


}

/// @nodoc
class $UiChatTypeCopyWith<$Res>  {
$UiChatTypeCopyWith(UiChatType _, $Res Function(UiChatType) __);
}



/// @nodoc


class UiChatType_HandleConnection extends UiChatType {
  const UiChatType_HandleConnection(this.field0): super._();
  

 final  UiUserHandle field0;

/// Create a copy of UiChatType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiChatType_HandleConnectionCopyWith<UiChatType_HandleConnection> get copyWith => _$UiChatType_HandleConnectionCopyWithImpl<UiChatType_HandleConnection>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatType_HandleConnection&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiChatType.handleConnection(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiChatType_HandleConnectionCopyWith<$Res> implements $UiChatTypeCopyWith<$Res> {
  factory $UiChatType_HandleConnectionCopyWith(UiChatType_HandleConnection value, $Res Function(UiChatType_HandleConnection) _then) = _$UiChatType_HandleConnectionCopyWithImpl;
@useResult
$Res call({
 UiUserHandle field0
});


$UiUserHandleCopyWith<$Res> get field0;

}
/// @nodoc
class _$UiChatType_HandleConnectionCopyWithImpl<$Res>
    implements $UiChatType_HandleConnectionCopyWith<$Res> {
  _$UiChatType_HandleConnectionCopyWithImpl(this._self, this._then);

  final UiChatType_HandleConnection _self;
  final $Res Function(UiChatType_HandleConnection) _then;

/// Create a copy of UiChatType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiChatType_HandleConnection(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserHandle,
  ));
}

/// Create a copy of UiChatType
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$UiUserHandleCopyWith<$Res> get field0 {
  
  return $UiUserHandleCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class UiChatType_Connection extends UiChatType {
  const UiChatType_Connection(this.field0): super._();
  

 final  UiUserProfile field0;

/// Create a copy of UiChatType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiChatType_ConnectionCopyWith<UiChatType_Connection> get copyWith => _$UiChatType_ConnectionCopyWithImpl<UiChatType_Connection>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatType_Connection&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiChatType.connection(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiChatType_ConnectionCopyWith<$Res> implements $UiChatTypeCopyWith<$Res> {
  factory $UiChatType_ConnectionCopyWith(UiChatType_Connection value, $Res Function(UiChatType_Connection) _then) = _$UiChatType_ConnectionCopyWithImpl;
@useResult
$Res call({
 UiUserProfile field0
});




}
/// @nodoc
class _$UiChatType_ConnectionCopyWithImpl<$Res>
    implements $UiChatType_ConnectionCopyWith<$Res> {
  _$UiChatType_ConnectionCopyWithImpl(this._self, this._then);

  final UiChatType_Connection _self;
  final $Res Function(UiChatType_Connection) _then;

/// Create a copy of UiChatType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiChatType_Connection(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserProfile,
  ));
}


}

/// @nodoc


class UiChatType_Group extends UiChatType {
  const UiChatType_Group(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiChatType_Group);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiChatType.group()';
}


}




/// @nodoc
mixin _$UiEventMessage {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEventMessage&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'UiEventMessage(field0: $field0)';
}


}

/// @nodoc
class $UiEventMessageCopyWith<$Res>  {
$UiEventMessageCopyWith(UiEventMessage _, $Res Function(UiEventMessage) __);
}



/// @nodoc


class UiEventMessage_System extends UiEventMessage {
  const UiEventMessage_System(this.field0): super._();
  

@override final  UiSystemMessage field0;

/// Create a copy of UiEventMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEventMessage_SystemCopyWith<UiEventMessage_System> get copyWith => _$UiEventMessage_SystemCopyWithImpl<UiEventMessage_System>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEventMessage_System&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiEventMessage.system(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiEventMessage_SystemCopyWith<$Res> implements $UiEventMessageCopyWith<$Res> {
  factory $UiEventMessage_SystemCopyWith(UiEventMessage_System value, $Res Function(UiEventMessage_System) _then) = _$UiEventMessage_SystemCopyWithImpl;
@useResult
$Res call({
 UiSystemMessage field0
});


$UiSystemMessageCopyWith<$Res> get field0;

}
/// @nodoc
class _$UiEventMessage_SystemCopyWithImpl<$Res>
    implements $UiEventMessage_SystemCopyWith<$Res> {
  _$UiEventMessage_SystemCopyWithImpl(this._self, this._then);

  final UiEventMessage_System _self;
  final $Res Function(UiEventMessage_System) _then;

/// Create a copy of UiEventMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiEventMessage_System(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiSystemMessage,
  ));
}

/// Create a copy of UiEventMessage
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$UiSystemMessageCopyWith<$Res> get field0 {
  
  return $UiSystemMessageCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class UiEventMessage_Error extends UiEventMessage {
  const UiEventMessage_Error(this.field0): super._();
  

@override final  UiErrorMessage field0;

/// Create a copy of UiEventMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEventMessage_ErrorCopyWith<UiEventMessage_Error> get copyWith => _$UiEventMessage_ErrorCopyWithImpl<UiEventMessage_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEventMessage_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiEventMessage.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiEventMessage_ErrorCopyWith<$Res> implements $UiEventMessageCopyWith<$Res> {
  factory $UiEventMessage_ErrorCopyWith(UiEventMessage_Error value, $Res Function(UiEventMessage_Error) _then) = _$UiEventMessage_ErrorCopyWithImpl;
@useResult
$Res call({
 UiErrorMessage field0
});




}
/// @nodoc
class _$UiEventMessage_ErrorCopyWithImpl<$Res>
    implements $UiEventMessage_ErrorCopyWith<$Res> {
  _$UiEventMessage_ErrorCopyWithImpl(this._self, this._then);

  final UiEventMessage_Error _self;
  final $Res Function(UiEventMessage_Error) _then;

/// Create a copy of UiEventMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiEventMessage_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiErrorMessage,
  ));
}


}

/// @nodoc
mixin _$UiMessage {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMessage&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'UiMessage(field0: $field0)';
}


}

/// @nodoc
class $UiMessageCopyWith<$Res>  {
$UiMessageCopyWith(UiMessage _, $Res Function(UiMessage) __);
}



/// @nodoc


class UiMessage_Content extends UiMessage {
  const UiMessage_Content(this.field0): super._();
  

@override final  UiContentMessage field0;

/// Create a copy of UiMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMessage_ContentCopyWith<UiMessage_Content> get copyWith => _$UiMessage_ContentCopyWithImpl<UiMessage_Content>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMessage_Content&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMessage.content(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMessage_ContentCopyWith<$Res> implements $UiMessageCopyWith<$Res> {
  factory $UiMessage_ContentCopyWith(UiMessage_Content value, $Res Function(UiMessage_Content) _then) = _$UiMessage_ContentCopyWithImpl;
@useResult
$Res call({
 UiContentMessage field0
});




}
/// @nodoc
class _$UiMessage_ContentCopyWithImpl<$Res>
    implements $UiMessage_ContentCopyWith<$Res> {
  _$UiMessage_ContentCopyWithImpl(this._self, this._then);

  final UiMessage_Content _self;
  final $Res Function(UiMessage_Content) _then;

/// Create a copy of UiMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMessage_Content(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiContentMessage,
  ));
}


}

/// @nodoc


class UiMessage_Display extends UiMessage {
  const UiMessage_Display(this.field0): super._();
  

@override final  UiEventMessage field0;

/// Create a copy of UiMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMessage_DisplayCopyWith<UiMessage_Display> get copyWith => _$UiMessage_DisplayCopyWithImpl<UiMessage_Display>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMessage_Display&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMessage.display(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMessage_DisplayCopyWith<$Res> implements $UiMessageCopyWith<$Res> {
  factory $UiMessage_DisplayCopyWith(UiMessage_Display value, $Res Function(UiMessage_Display) _then) = _$UiMessage_DisplayCopyWithImpl;
@useResult
$Res call({
 UiEventMessage field0
});


$UiEventMessageCopyWith<$Res> get field0;

}
/// @nodoc
class _$UiMessage_DisplayCopyWithImpl<$Res>
    implements $UiMessage_DisplayCopyWith<$Res> {
  _$UiMessage_DisplayCopyWithImpl(this._self, this._then);

  final UiMessage_Display _self;
  final $Res Function(UiMessage_Display) _then;

/// Create a copy of UiMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMessage_Display(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiEventMessage,
  ));
}

/// Create a copy of UiMessage
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$UiEventMessageCopyWith<$Res> get field0 {
  
  return $UiEventMessageCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc
mixin _$UiMessageDraft {

 String get message; MessageId? get editingId; DateTime get updatedAt; UiMessageDraftSource get source;
/// Create a copy of UiMessageDraft
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMessageDraftCopyWith<UiMessageDraft> get copyWith => _$UiMessageDraftCopyWithImpl<UiMessageDraft>(this as UiMessageDraft, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMessageDraft&&(identical(other.message, message) || other.message == message)&&(identical(other.editingId, editingId) || other.editingId == editingId)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,message,editingId,updatedAt,source);

@override
String toString() {
  return 'UiMessageDraft(message: $message, editingId: $editingId, updatedAt: $updatedAt, source: $source)';
}


}

/// @nodoc
abstract mixin class $UiMessageDraftCopyWith<$Res>  {
  factory $UiMessageDraftCopyWith(UiMessageDraft value, $Res Function(UiMessageDraft) _then) = _$UiMessageDraftCopyWithImpl;
@useResult
$Res call({
 String message, MessageId? editingId, DateTime updatedAt, UiMessageDraftSource source
});




}
/// @nodoc
class _$UiMessageDraftCopyWithImpl<$Res>
    implements $UiMessageDraftCopyWith<$Res> {
  _$UiMessageDraftCopyWithImpl(this._self, this._then);

  final UiMessageDraft _self;
  final $Res Function(UiMessageDraft) _then;

/// Create a copy of UiMessageDraft
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? message = null,Object? editingId = freezed,Object? updatedAt = null,Object? source = null,}) {
  return _then(_self.copyWith(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,editingId: freezed == editingId ? _self.editingId : editingId // ignore: cast_nullable_to_non_nullable
as MessageId?,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as DateTime,source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as UiMessageDraftSource,
  ));
}

}



/// @nodoc


class _UiMessageDraft implements UiMessageDraft {
  const _UiMessageDraft({required this.message, this.editingId, required this.updatedAt, required this.source});
  

@override final  String message;
@override final  MessageId? editingId;
@override final  DateTime updatedAt;
@override final  UiMessageDraftSource source;

/// Create a copy of UiMessageDraft
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$UiMessageDraftCopyWith<_UiMessageDraft> get copyWith => __$UiMessageDraftCopyWithImpl<_UiMessageDraft>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _UiMessageDraft&&(identical(other.message, message) || other.message == message)&&(identical(other.editingId, editingId) || other.editingId == editingId)&&(identical(other.updatedAt, updatedAt) || other.updatedAt == updatedAt)&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,message,editingId,updatedAt,source);

@override
String toString() {
  return 'UiMessageDraft(message: $message, editingId: $editingId, updatedAt: $updatedAt, source: $source)';
}


}

/// @nodoc
abstract mixin class _$UiMessageDraftCopyWith<$Res> implements $UiMessageDraftCopyWith<$Res> {
  factory _$UiMessageDraftCopyWith(_UiMessageDraft value, $Res Function(_UiMessageDraft) _then) = __$UiMessageDraftCopyWithImpl;
@override @useResult
$Res call({
 String message, MessageId? editingId, DateTime updatedAt, UiMessageDraftSource source
});




}
/// @nodoc
class __$UiMessageDraftCopyWithImpl<$Res>
    implements _$UiMessageDraftCopyWith<$Res> {
  __$UiMessageDraftCopyWithImpl(this._self, this._then);

  final _UiMessageDraft _self;
  final $Res Function(_UiMessageDraft) _then;

/// Create a copy of UiMessageDraft
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? message = null,Object? editingId = freezed,Object? updatedAt = null,Object? source = null,}) {
  return _then(_UiMessageDraft(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,editingId: freezed == editingId ? _self.editingId : editingId // ignore: cast_nullable_to_non_nullable
as MessageId?,updatedAt: null == updatedAt ? _self.updatedAt : updatedAt // ignore: cast_nullable_to_non_nullable
as DateTime,source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as UiMessageDraftSource,
  ));
}


}

/// @nodoc
mixin _$UiSystemMessage {

 UiUserId get field0; UiUserId get field1;
/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiSystemMessageCopyWith<UiSystemMessage> get copyWith => _$UiSystemMessageCopyWithImpl<UiSystemMessage>(this as UiSystemMessage, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiSystemMessage&&(identical(other.field0, field0) || other.field0 == field0)&&(identical(other.field1, field1) || other.field1 == field1));
}


@override
int get hashCode => Object.hash(runtimeType,field0,field1);

@override
String toString() {
  return 'UiSystemMessage(field0: $field0, field1: $field1)';
}


}

/// @nodoc
abstract mixin class $UiSystemMessageCopyWith<$Res>  {
  factory $UiSystemMessageCopyWith(UiSystemMessage value, $Res Function(UiSystemMessage) _then) = _$UiSystemMessageCopyWithImpl;
@useResult
$Res call({
 UiUserId field0, UiUserId field1
});




}
/// @nodoc
class _$UiSystemMessageCopyWithImpl<$Res>
    implements $UiSystemMessageCopyWith<$Res> {
  _$UiSystemMessageCopyWithImpl(this._self, this._then);

  final UiSystemMessage _self;
  final $Res Function(UiSystemMessage) _then;

/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? field0 = null,Object? field1 = null,}) {
  return _then(_self.copyWith(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserId,field1: null == field1 ? _self.field1 : field1 // ignore: cast_nullable_to_non_nullable
as UiUserId,
  ));
}

}



/// @nodoc


class UiSystemMessage_Add extends UiSystemMessage {
  const UiSystemMessage_Add(this.field0, this.field1): super._();
  

@override final  UiUserId field0;
@override final  UiUserId field1;

/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiSystemMessage_AddCopyWith<UiSystemMessage_Add> get copyWith => _$UiSystemMessage_AddCopyWithImpl<UiSystemMessage_Add>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiSystemMessage_Add&&(identical(other.field0, field0) || other.field0 == field0)&&(identical(other.field1, field1) || other.field1 == field1));
}


@override
int get hashCode => Object.hash(runtimeType,field0,field1);

@override
String toString() {
  return 'UiSystemMessage.add(field0: $field0, field1: $field1)';
}


}

/// @nodoc
abstract mixin class $UiSystemMessage_AddCopyWith<$Res> implements $UiSystemMessageCopyWith<$Res> {
  factory $UiSystemMessage_AddCopyWith(UiSystemMessage_Add value, $Res Function(UiSystemMessage_Add) _then) = _$UiSystemMessage_AddCopyWithImpl;
@override @useResult
$Res call({
 UiUserId field0, UiUserId field1
});




}
/// @nodoc
class _$UiSystemMessage_AddCopyWithImpl<$Res>
    implements $UiSystemMessage_AddCopyWith<$Res> {
  _$UiSystemMessage_AddCopyWithImpl(this._self, this._then);

  final UiSystemMessage_Add _self;
  final $Res Function(UiSystemMessage_Add) _then;

/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,Object? field1 = null,}) {
  return _then(UiSystemMessage_Add(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserId,null == field1 ? _self.field1 : field1 // ignore: cast_nullable_to_non_nullable
as UiUserId,
  ));
}


}

/// @nodoc


class UiSystemMessage_Remove extends UiSystemMessage {
  const UiSystemMessage_Remove(this.field0, this.field1): super._();
  

@override final  UiUserId field0;
@override final  UiUserId field1;

/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiSystemMessage_RemoveCopyWith<UiSystemMessage_Remove> get copyWith => _$UiSystemMessage_RemoveCopyWithImpl<UiSystemMessage_Remove>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiSystemMessage_Remove&&(identical(other.field0, field0) || other.field0 == field0)&&(identical(other.field1, field1) || other.field1 == field1));
}


@override
int get hashCode => Object.hash(runtimeType,field0,field1);

@override
String toString() {
  return 'UiSystemMessage.remove(field0: $field0, field1: $field1)';
}


}

/// @nodoc
abstract mixin class $UiSystemMessage_RemoveCopyWith<$Res> implements $UiSystemMessageCopyWith<$Res> {
  factory $UiSystemMessage_RemoveCopyWith(UiSystemMessage_Remove value, $Res Function(UiSystemMessage_Remove) _then) = _$UiSystemMessage_RemoveCopyWithImpl;
@override @useResult
$Res call({
 UiUserId field0, UiUserId field1
});




}
/// @nodoc
class _$UiSystemMessage_RemoveCopyWithImpl<$Res>
    implements $UiSystemMessage_RemoveCopyWith<$Res> {
  _$UiSystemMessage_RemoveCopyWithImpl(this._self, this._then);

  final UiSystemMessage_Remove _self;
  final $Res Function(UiSystemMessage_Remove) _then;

/// Create a copy of UiSystemMessage
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,Object? field1 = null,}) {
  return _then(UiSystemMessage_Remove(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserId,null == field1 ? _self.field1 : field1 // ignore: cast_nullable_to_non_nullable
as UiUserId,
  ));
}


}

/// @nodoc
mixin _$UiUserHandle {

 String get plaintext;
/// Create a copy of UiUserHandle
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiUserHandleCopyWith<UiUserHandle> get copyWith => _$UiUserHandleCopyWithImpl<UiUserHandle>(this as UiUserHandle, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiUserHandle&&(identical(other.plaintext, plaintext) || other.plaintext == plaintext));
}


@override
int get hashCode => Object.hash(runtimeType,plaintext);

@override
String toString() {
  return 'UiUserHandle(plaintext: $plaintext)';
}


}

/// @nodoc
abstract mixin class $UiUserHandleCopyWith<$Res>  {
  factory $UiUserHandleCopyWith(UiUserHandle value, $Res Function(UiUserHandle) _then) = _$UiUserHandleCopyWithImpl;
@useResult
$Res call({
 String plaintext
});




}
/// @nodoc
class _$UiUserHandleCopyWithImpl<$Res>
    implements $UiUserHandleCopyWith<$Res> {
  _$UiUserHandleCopyWithImpl(this._self, this._then);

  final UiUserHandle _self;
  final $Res Function(UiUserHandle) _then;

/// Create a copy of UiUserHandle
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? plaintext = null,}) {
  return _then(_self.copyWith(
plaintext: null == plaintext ? _self.plaintext : plaintext // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}



/// @nodoc


class _UiUserHandle extends UiUserHandle {
  const _UiUserHandle({required this.plaintext}): super._();
  

@override final  String plaintext;

/// Create a copy of UiUserHandle
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$UiUserHandleCopyWith<_UiUserHandle> get copyWith => __$UiUserHandleCopyWithImpl<_UiUserHandle>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _UiUserHandle&&(identical(other.plaintext, plaintext) || other.plaintext == plaintext));
}


@override
int get hashCode => Object.hash(runtimeType,plaintext);

@override
String toString() {
  return 'UiUserHandle(plaintext: $plaintext)';
}


}

/// @nodoc
abstract mixin class _$UiUserHandleCopyWith<$Res> implements $UiUserHandleCopyWith<$Res> {
  factory _$UiUserHandleCopyWith(_UiUserHandle value, $Res Function(_UiUserHandle) _then) = __$UiUserHandleCopyWithImpl;
@override @useResult
$Res call({
 String plaintext
});




}
/// @nodoc
class __$UiUserHandleCopyWithImpl<$Res>
    implements _$UiUserHandleCopyWith<$Res> {
  __$UiUserHandleCopyWithImpl(this._self, this._then);

  final _UiUserHandle _self;
  final $Res Function(_UiUserHandle) _then;

/// Create a copy of UiUserHandle
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? plaintext = null,}) {
  return _then(_UiUserHandle(
plaintext: null == plaintext ? _self.plaintext : plaintext // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
