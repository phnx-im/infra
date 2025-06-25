// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiConversationStatus {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationStatus);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiConversationStatus()';
}


}

/// @nodoc
class $UiConversationStatusCopyWith<$Res>  {
$UiConversationStatusCopyWith(UiConversationStatus _, $Res Function(UiConversationStatus) __);
}


/// @nodoc


class UiConversationStatus_Inactive extends UiConversationStatus {
  const UiConversationStatus_Inactive(this.field0): super._();
  

 final  UiInactiveConversation field0;

/// Create a copy of UiConversationStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiConversationStatus_InactiveCopyWith<UiConversationStatus_Inactive> get copyWith => _$UiConversationStatus_InactiveCopyWithImpl<UiConversationStatus_Inactive>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationStatus_Inactive&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiConversationStatus.inactive(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiConversationStatus_InactiveCopyWith<$Res> implements $UiConversationStatusCopyWith<$Res> {
  factory $UiConversationStatus_InactiveCopyWith(UiConversationStatus_Inactive value, $Res Function(UiConversationStatus_Inactive) _then) = _$UiConversationStatus_InactiveCopyWithImpl;
@useResult
$Res call({
 UiInactiveConversation field0
});




}
/// @nodoc
class _$UiConversationStatus_InactiveCopyWithImpl<$Res>
    implements $UiConversationStatus_InactiveCopyWith<$Res> {
  _$UiConversationStatus_InactiveCopyWithImpl(this._self, this._then);

  final UiConversationStatus_Inactive _self;
  final $Res Function(UiConversationStatus_Inactive) _then;

/// Create a copy of UiConversationStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiConversationStatus_Inactive(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiInactiveConversation,
  ));
}


}

/// @nodoc


class UiConversationStatus_Active extends UiConversationStatus {
  const UiConversationStatus_Active(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationStatus_Active);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiConversationStatus.active()';
}


}




/// @nodoc
mixin _$UiConversationType {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationType);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiConversationType()';
}


}

/// @nodoc
class $UiConversationTypeCopyWith<$Res>  {
$UiConversationTypeCopyWith(UiConversationType _, $Res Function(UiConversationType) __);
}


/// @nodoc


class UiConversationType_HandleConnection extends UiConversationType {
  const UiConversationType_HandleConnection(this.field0): super._();
  

 final  UiUserHandle field0;

/// Create a copy of UiConversationType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiConversationType_HandleConnectionCopyWith<UiConversationType_HandleConnection> get copyWith => _$UiConversationType_HandleConnectionCopyWithImpl<UiConversationType_HandleConnection>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationType_HandleConnection&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiConversationType.handleConnection(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiConversationType_HandleConnectionCopyWith<$Res> implements $UiConversationTypeCopyWith<$Res> {
  factory $UiConversationType_HandleConnectionCopyWith(UiConversationType_HandleConnection value, $Res Function(UiConversationType_HandleConnection) _then) = _$UiConversationType_HandleConnectionCopyWithImpl;
@useResult
$Res call({
 UiUserHandle field0
});


$UiUserHandleCopyWith<$Res> get field0;

}
/// @nodoc
class _$UiConversationType_HandleConnectionCopyWithImpl<$Res>
    implements $UiConversationType_HandleConnectionCopyWith<$Res> {
  _$UiConversationType_HandleConnectionCopyWithImpl(this._self, this._then);

  final UiConversationType_HandleConnection _self;
  final $Res Function(UiConversationType_HandleConnection) _then;

/// Create a copy of UiConversationType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiConversationType_HandleConnection(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserHandle,
  ));
}

/// Create a copy of UiConversationType
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


class UiConversationType_Connection extends UiConversationType {
  const UiConversationType_Connection(this.field0): super._();
  

 final  UiUserProfile field0;

/// Create a copy of UiConversationType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiConversationType_ConnectionCopyWith<UiConversationType_Connection> get copyWith => _$UiConversationType_ConnectionCopyWithImpl<UiConversationType_Connection>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationType_Connection&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiConversationType.connection(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiConversationType_ConnectionCopyWith<$Res> implements $UiConversationTypeCopyWith<$Res> {
  factory $UiConversationType_ConnectionCopyWith(UiConversationType_Connection value, $Res Function(UiConversationType_Connection) _then) = _$UiConversationType_ConnectionCopyWithImpl;
@useResult
$Res call({
 UiUserProfile field0
});




}
/// @nodoc
class _$UiConversationType_ConnectionCopyWithImpl<$Res>
    implements $UiConversationType_ConnectionCopyWith<$Res> {
  _$UiConversationType_ConnectionCopyWithImpl(this._self, this._then);

  final UiConversationType_Connection _self;
  final $Res Function(UiConversationType_Connection) _then;

/// Create a copy of UiConversationType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiConversationType_Connection(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiUserProfile,
  ));
}


}

/// @nodoc


class UiConversationType_Group extends UiConversationType {
  const UiConversationType_Group(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiConversationType_Group);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiConversationType.group()';
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
