// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$UiConversationStatus {}

/// @nodoc
abstract class $UiConversationStatusCopyWith<$Res> {
  factory $UiConversationStatusCopyWith(
    UiConversationStatus value,
    $Res Function(UiConversationStatus) then,
  ) = _$UiConversationStatusCopyWithImpl<$Res, UiConversationStatus>;
}

/// @nodoc
class _$UiConversationStatusCopyWithImpl<
  $Res,
  $Val extends UiConversationStatus
>
    implements $UiConversationStatusCopyWith<$Res> {
  _$UiConversationStatusCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiConversationStatus
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$UiConversationStatus_InactiveImplCopyWith<$Res> {
  factory _$$UiConversationStatus_InactiveImplCopyWith(
    _$UiConversationStatus_InactiveImpl value,
    $Res Function(_$UiConversationStatus_InactiveImpl) then,
  ) = __$$UiConversationStatus_InactiveImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiInactiveConversation field0});
}

/// @nodoc
class __$$UiConversationStatus_InactiveImplCopyWithImpl<$Res>
    extends
        _$UiConversationStatusCopyWithImpl<
          $Res,
          _$UiConversationStatus_InactiveImpl
        >
    implements _$$UiConversationStatus_InactiveImplCopyWith<$Res> {
  __$$UiConversationStatus_InactiveImplCopyWithImpl(
    _$UiConversationStatus_InactiveImpl _value,
    $Res Function(_$UiConversationStatus_InactiveImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiConversationStatus
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiConversationStatus_InactiveImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiInactiveConversation,
      ),
    );
  }
}

/// @nodoc

class _$UiConversationStatus_InactiveImpl
    extends UiConversationStatus_Inactive {
  const _$UiConversationStatus_InactiveImpl(this.field0) : super._();

  @override
  final UiInactiveConversation field0;

  @override
  String toString() {
    return 'UiConversationStatus.inactive(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiConversationStatus_InactiveImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiConversationStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationStatus_InactiveImplCopyWith<
    _$UiConversationStatus_InactiveImpl
  >
  get copyWith => __$$UiConversationStatus_InactiveImplCopyWithImpl<
    _$UiConversationStatus_InactiveImpl
  >(this, _$identity);
}

abstract class UiConversationStatus_Inactive extends UiConversationStatus {
  const factory UiConversationStatus_Inactive(
    final UiInactiveConversation field0,
  ) = _$UiConversationStatus_InactiveImpl;
  const UiConversationStatus_Inactive._() : super._();

  UiInactiveConversation get field0;

  /// Create a copy of UiConversationStatus
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiConversationStatus_InactiveImplCopyWith<
    _$UiConversationStatus_InactiveImpl
  >
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationStatus_ActiveImplCopyWith<$Res> {
  factory _$$UiConversationStatus_ActiveImplCopyWith(
    _$UiConversationStatus_ActiveImpl value,
    $Res Function(_$UiConversationStatus_ActiveImpl) then,
  ) = __$$UiConversationStatus_ActiveImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$UiConversationStatus_ActiveImplCopyWithImpl<$Res>
    extends
        _$UiConversationStatusCopyWithImpl<
          $Res,
          _$UiConversationStatus_ActiveImpl
        >
    implements _$$UiConversationStatus_ActiveImplCopyWith<$Res> {
  __$$UiConversationStatus_ActiveImplCopyWithImpl(
    _$UiConversationStatus_ActiveImpl _value,
    $Res Function(_$UiConversationStatus_ActiveImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiConversationStatus
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$UiConversationStatus_ActiveImpl extends UiConversationStatus_Active {
  const _$UiConversationStatus_ActiveImpl() : super._();

  @override
  String toString() {
    return 'UiConversationStatus.active()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiConversationStatus_ActiveImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;
}

abstract class UiConversationStatus_Active extends UiConversationStatus {
  const factory UiConversationStatus_Active() =
      _$UiConversationStatus_ActiveImpl;
  const UiConversationStatus_Active._() : super._();
}

/// @nodoc
mixin _$UiConversationType {}

/// @nodoc
abstract class $UiConversationTypeCopyWith<$Res> {
  factory $UiConversationTypeCopyWith(
    UiConversationType value,
    $Res Function(UiConversationType) then,
  ) = _$UiConversationTypeCopyWithImpl<$Res, UiConversationType>;
}

/// @nodoc
class _$UiConversationTypeCopyWithImpl<$Res, $Val extends UiConversationType>
    implements $UiConversationTypeCopyWith<$Res> {
  _$UiConversationTypeCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$UiConversationType_UnconfirmedConnectionImplCopyWith<$Res> {
  factory _$$UiConversationType_UnconfirmedConnectionImplCopyWith(
    _$UiConversationType_UnconfirmedConnectionImpl value,
    $Res Function(_$UiConversationType_UnconfirmedConnectionImpl) then,
  ) = __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiUserProfile field0});
}

/// @nodoc
class __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<$Res>
    extends
        _$UiConversationTypeCopyWithImpl<
          $Res,
          _$UiConversationType_UnconfirmedConnectionImpl
        >
    implements _$$UiConversationType_UnconfirmedConnectionImplCopyWith<$Res> {
  __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl(
    _$UiConversationType_UnconfirmedConnectionImpl _value,
    $Res Function(_$UiConversationType_UnconfirmedConnectionImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiConversationType_UnconfirmedConnectionImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiUserProfile,
      ),
    );
  }
}

/// @nodoc

class _$UiConversationType_UnconfirmedConnectionImpl
    extends UiConversationType_UnconfirmedConnection {
  const _$UiConversationType_UnconfirmedConnectionImpl(this.field0) : super._();

  @override
  final UiUserProfile field0;

  @override
  String toString() {
    return 'UiConversationType.unconfirmedConnection(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiConversationType_UnconfirmedConnectionImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationType_UnconfirmedConnectionImplCopyWith<
    _$UiConversationType_UnconfirmedConnectionImpl
  >
  get copyWith => __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<
    _$UiConversationType_UnconfirmedConnectionImpl
  >(this, _$identity);
}

abstract class UiConversationType_UnconfirmedConnection
    extends UiConversationType {
  const factory UiConversationType_UnconfirmedConnection(
    final UiUserProfile field0,
  ) = _$UiConversationType_UnconfirmedConnectionImpl;
  const UiConversationType_UnconfirmedConnection._() : super._();

  UiUserProfile get field0;

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiConversationType_UnconfirmedConnectionImplCopyWith<
    _$UiConversationType_UnconfirmedConnectionImpl
  >
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationType_ConnectionImplCopyWith<$Res> {
  factory _$$UiConversationType_ConnectionImplCopyWith(
    _$UiConversationType_ConnectionImpl value,
    $Res Function(_$UiConversationType_ConnectionImpl) then,
  ) = __$$UiConversationType_ConnectionImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiUserProfile field0});
}

/// @nodoc
class __$$UiConversationType_ConnectionImplCopyWithImpl<$Res>
    extends
        _$UiConversationTypeCopyWithImpl<
          $Res,
          _$UiConversationType_ConnectionImpl
        >
    implements _$$UiConversationType_ConnectionImplCopyWith<$Res> {
  __$$UiConversationType_ConnectionImplCopyWithImpl(
    _$UiConversationType_ConnectionImpl _value,
    $Res Function(_$UiConversationType_ConnectionImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiConversationType_ConnectionImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiUserProfile,
      ),
    );
  }
}

/// @nodoc

class _$UiConversationType_ConnectionImpl
    extends UiConversationType_Connection {
  const _$UiConversationType_ConnectionImpl(this.field0) : super._();

  @override
  final UiUserProfile field0;

  @override
  String toString() {
    return 'UiConversationType.connection(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiConversationType_ConnectionImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationType_ConnectionImplCopyWith<
    _$UiConversationType_ConnectionImpl
  >
  get copyWith => __$$UiConversationType_ConnectionImplCopyWithImpl<
    _$UiConversationType_ConnectionImpl
  >(this, _$identity);
}

abstract class UiConversationType_Connection extends UiConversationType {
  const factory UiConversationType_Connection(final UiUserProfile field0) =
      _$UiConversationType_ConnectionImpl;
  const UiConversationType_Connection._() : super._();

  UiUserProfile get field0;

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiConversationType_ConnectionImplCopyWith<
    _$UiConversationType_ConnectionImpl
  >
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationType_GroupImplCopyWith<$Res> {
  factory _$$UiConversationType_GroupImplCopyWith(
    _$UiConversationType_GroupImpl value,
    $Res Function(_$UiConversationType_GroupImpl) then,
  ) = __$$UiConversationType_GroupImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$UiConversationType_GroupImplCopyWithImpl<$Res>
    extends
        _$UiConversationTypeCopyWithImpl<$Res, _$UiConversationType_GroupImpl>
    implements _$$UiConversationType_GroupImplCopyWith<$Res> {
  __$$UiConversationType_GroupImplCopyWithImpl(
    _$UiConversationType_GroupImpl _value,
    $Res Function(_$UiConversationType_GroupImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiConversationType
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$UiConversationType_GroupImpl extends UiConversationType_Group {
  const _$UiConversationType_GroupImpl() : super._();

  @override
  String toString() {
    return 'UiConversationType.group()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiConversationType_GroupImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;
}

abstract class UiConversationType_Group extends UiConversationType {
  const factory UiConversationType_Group() = _$UiConversationType_GroupImpl;
  const UiConversationType_Group._() : super._();
}

/// @nodoc
mixin _$UiEventMessage {
  Object get field0 => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiEventMessageCopyWith<$Res> {
  factory $UiEventMessageCopyWith(
    UiEventMessage value,
    $Res Function(UiEventMessage) then,
  ) = _$UiEventMessageCopyWithImpl<$Res, UiEventMessage>;
}

/// @nodoc
class _$UiEventMessageCopyWithImpl<$Res, $Val extends UiEventMessage>
    implements $UiEventMessageCopyWith<$Res> {
  _$UiEventMessageCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$UiEventMessage_SystemImplCopyWith<$Res> {
  factory _$$UiEventMessage_SystemImplCopyWith(
    _$UiEventMessage_SystemImpl value,
    $Res Function(_$UiEventMessage_SystemImpl) then,
  ) = __$$UiEventMessage_SystemImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiSystemMessage field0});
}

/// @nodoc
class __$$UiEventMessage_SystemImplCopyWithImpl<$Res>
    extends _$UiEventMessageCopyWithImpl<$Res, _$UiEventMessage_SystemImpl>
    implements _$$UiEventMessage_SystemImplCopyWith<$Res> {
  __$$UiEventMessage_SystemImplCopyWithImpl(
    _$UiEventMessage_SystemImpl _value,
    $Res Function(_$UiEventMessage_SystemImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiEventMessage_SystemImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiSystemMessage,
      ),
    );
  }
}

/// @nodoc

class _$UiEventMessage_SystemImpl extends UiEventMessage_System {
  const _$UiEventMessage_SystemImpl(this.field0) : super._();

  @override
  final UiSystemMessage field0;

  @override
  String toString() {
    return 'UiEventMessage.system(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiEventMessage_SystemImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiEventMessage_SystemImplCopyWith<_$UiEventMessage_SystemImpl>
  get copyWith =>
      __$$UiEventMessage_SystemImplCopyWithImpl<_$UiEventMessage_SystemImpl>(
        this,
        _$identity,
      );
}

abstract class UiEventMessage_System extends UiEventMessage {
  const factory UiEventMessage_System(final UiSystemMessage field0) =
      _$UiEventMessage_SystemImpl;
  const UiEventMessage_System._() : super._();

  @override
  UiSystemMessage get field0;

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiEventMessage_SystemImplCopyWith<_$UiEventMessage_SystemImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiEventMessage_ErrorImplCopyWith<$Res> {
  factory _$$UiEventMessage_ErrorImplCopyWith(
    _$UiEventMessage_ErrorImpl value,
    $Res Function(_$UiEventMessage_ErrorImpl) then,
  ) = __$$UiEventMessage_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiErrorMessage field0});
}

/// @nodoc
class __$$UiEventMessage_ErrorImplCopyWithImpl<$Res>
    extends _$UiEventMessageCopyWithImpl<$Res, _$UiEventMessage_ErrorImpl>
    implements _$$UiEventMessage_ErrorImplCopyWith<$Res> {
  __$$UiEventMessage_ErrorImplCopyWithImpl(
    _$UiEventMessage_ErrorImpl _value,
    $Res Function(_$UiEventMessage_ErrorImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiEventMessage_ErrorImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiErrorMessage,
      ),
    );
  }
}

/// @nodoc

class _$UiEventMessage_ErrorImpl extends UiEventMessage_Error {
  const _$UiEventMessage_ErrorImpl(this.field0) : super._();

  @override
  final UiErrorMessage field0;

  @override
  String toString() {
    return 'UiEventMessage.error(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiEventMessage_ErrorImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiEventMessage_ErrorImplCopyWith<_$UiEventMessage_ErrorImpl>
  get copyWith =>
      __$$UiEventMessage_ErrorImplCopyWithImpl<_$UiEventMessage_ErrorImpl>(
        this,
        _$identity,
      );
}

abstract class UiEventMessage_Error extends UiEventMessage {
  const factory UiEventMessage_Error(final UiErrorMessage field0) =
      _$UiEventMessage_ErrorImpl;
  const UiEventMessage_Error._() : super._();

  @override
  UiErrorMessage get field0;

  /// Create a copy of UiEventMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiEventMessage_ErrorImplCopyWith<_$UiEventMessage_ErrorImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UiMessage {
  Object get field0 => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiMessageCopyWith<$Res> {
  factory $UiMessageCopyWith(UiMessage value, $Res Function(UiMessage) then) =
      _$UiMessageCopyWithImpl<$Res, UiMessage>;
}

/// @nodoc
class _$UiMessageCopyWithImpl<$Res, $Val extends UiMessage>
    implements $UiMessageCopyWith<$Res> {
  _$UiMessageCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$UiMessage_ContentImplCopyWith<$Res> {
  factory _$$UiMessage_ContentImplCopyWith(
    _$UiMessage_ContentImpl value,
    $Res Function(_$UiMessage_ContentImpl) then,
  ) = __$$UiMessage_ContentImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiContentMessage field0});
}

/// @nodoc
class __$$UiMessage_ContentImplCopyWithImpl<$Res>
    extends _$UiMessageCopyWithImpl<$Res, _$UiMessage_ContentImpl>
    implements _$$UiMessage_ContentImplCopyWith<$Res> {
  __$$UiMessage_ContentImplCopyWithImpl(
    _$UiMessage_ContentImpl _value,
    $Res Function(_$UiMessage_ContentImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiMessage_ContentImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiContentMessage,
      ),
    );
  }
}

/// @nodoc

class _$UiMessage_ContentImpl extends UiMessage_Content {
  const _$UiMessage_ContentImpl(this.field0) : super._();

  @override
  final UiContentMessage field0;

  @override
  String toString() {
    return 'UiMessage.content(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiMessage_ContentImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMessage_ContentImplCopyWith<_$UiMessage_ContentImpl> get copyWith =>
      __$$UiMessage_ContentImplCopyWithImpl<_$UiMessage_ContentImpl>(
        this,
        _$identity,
      );
}

abstract class UiMessage_Content extends UiMessage {
  const factory UiMessage_Content(final UiContentMessage field0) =
      _$UiMessage_ContentImpl;
  const UiMessage_Content._() : super._();

  @override
  UiContentMessage get field0;

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiMessage_ContentImplCopyWith<_$UiMessage_ContentImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiMessage_DisplayImplCopyWith<$Res> {
  factory _$$UiMessage_DisplayImplCopyWith(
    _$UiMessage_DisplayImpl value,
    $Res Function(_$UiMessage_DisplayImpl) then,
  ) = __$$UiMessage_DisplayImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiEventMessage field0});

  $UiEventMessageCopyWith<$Res> get field0;
}

/// @nodoc
class __$$UiMessage_DisplayImplCopyWithImpl<$Res>
    extends _$UiMessageCopyWithImpl<$Res, _$UiMessage_DisplayImpl>
    implements _$$UiMessage_DisplayImplCopyWith<$Res> {
  __$$UiMessage_DisplayImplCopyWithImpl(
    _$UiMessage_DisplayImpl _value,
    $Res Function(_$UiMessage_DisplayImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$UiMessage_DisplayImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                as UiEventMessage,
      ),
    );
  }

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $UiEventMessageCopyWith<$Res> get field0 {
    return $UiEventMessageCopyWith<$Res>(_value.field0, (value) {
      return _then(_value.copyWith(field0: value));
    });
  }
}

/// @nodoc

class _$UiMessage_DisplayImpl extends UiMessage_Display {
  const _$UiMessage_DisplayImpl(this.field0) : super._();

  @override
  final UiEventMessage field0;

  @override
  String toString() {
    return 'UiMessage.display(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiMessage_DisplayImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMessage_DisplayImplCopyWith<_$UiMessage_DisplayImpl> get copyWith =>
      __$$UiMessage_DisplayImplCopyWithImpl<_$UiMessage_DisplayImpl>(
        this,
        _$identity,
      );
}

abstract class UiMessage_Display extends UiMessage {
  const factory UiMessage_Display(final UiEventMessage field0) =
      _$UiMessage_DisplayImpl;
  const UiMessage_Display._() : super._();

  @override
  UiEventMessage get field0;

  /// Create a copy of UiMessage
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiMessage_DisplayImplCopyWith<_$UiMessage_DisplayImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UiMimiContent {
  Uint8List? get replaces => throw _privateConstructorUsedError;
  Uint8List get topicId => throw _privateConstructorUsedError;
  Uint8List? get inReplyTo => throw _privateConstructorUsedError;
  String get plainBody => throw _privateConstructorUsedError;
  MessageContent get content => throw _privateConstructorUsedError;

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $UiMimiContentCopyWith<UiMimiContent> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiMimiContentCopyWith<$Res> {
  factory $UiMimiContentCopyWith(
    UiMimiContent value,
    $Res Function(UiMimiContent) then,
  ) = _$UiMimiContentCopyWithImpl<$Res, UiMimiContent>;
  @useResult
  $Res call({
    Uint8List? replaces,
    Uint8List topicId,
    Uint8List? inReplyTo,
    String plainBody,
    MessageContent content,
  });

  $MessageContentCopyWith<$Res> get content;
}

/// @nodoc
class _$UiMimiContentCopyWithImpl<$Res, $Val extends UiMimiContent>
    implements $UiMimiContentCopyWith<$Res> {
  _$UiMimiContentCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? replaces = freezed,
    Object? topicId = null,
    Object? inReplyTo = freezed,
    Object? plainBody = null,
    Object? content = null,
  }) {
    return _then(
      _value.copyWith(
            replaces:
                freezed == replaces
                    ? _value.replaces
                    : replaces // ignore: cast_nullable_to_non_nullable
                        as Uint8List?,
            topicId:
                null == topicId
                    ? _value.topicId
                    : topicId // ignore: cast_nullable_to_non_nullable
                        as Uint8List,
            inReplyTo:
                freezed == inReplyTo
                    ? _value.inReplyTo
                    : inReplyTo // ignore: cast_nullable_to_non_nullable
                        as Uint8List?,
            plainBody:
                null == plainBody
                    ? _value.plainBody
                    : plainBody // ignore: cast_nullable_to_non_nullable
                        as String,
            content:
                null == content
                    ? _value.content
                    : content // ignore: cast_nullable_to_non_nullable
                        as MessageContent,
          )
          as $Val,
    );
  }

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $MessageContentCopyWith<$Res> get content {
    return $MessageContentCopyWith<$Res>(_value.content, (value) {
      return _then(_value.copyWith(content: value) as $Val);
    });
  }
}

/// @nodoc
abstract class _$$UiMimiContentImplCopyWith<$Res>
    implements $UiMimiContentCopyWith<$Res> {
  factory _$$UiMimiContentImplCopyWith(
    _$UiMimiContentImpl value,
    $Res Function(_$UiMimiContentImpl) then,
  ) = __$$UiMimiContentImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({
    Uint8List? replaces,
    Uint8List topicId,
    Uint8List? inReplyTo,
    String plainBody,
    MessageContent content,
  });

  @override
  $MessageContentCopyWith<$Res> get content;
}

/// @nodoc
class __$$UiMimiContentImplCopyWithImpl<$Res>
    extends _$UiMimiContentCopyWithImpl<$Res, _$UiMimiContentImpl>
    implements _$$UiMimiContentImplCopyWith<$Res> {
  __$$UiMimiContentImplCopyWithImpl(
    _$UiMimiContentImpl _value,
    $Res Function(_$UiMimiContentImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? replaces = freezed,
    Object? topicId = null,
    Object? inReplyTo = freezed,
    Object? plainBody = null,
    Object? content = null,
  }) {
    return _then(
      _$UiMimiContentImpl(
        replaces:
            freezed == replaces
                ? _value.replaces
                : replaces // ignore: cast_nullable_to_non_nullable
                    as Uint8List?,
        topicId:
            null == topicId
                ? _value.topicId
                : topicId // ignore: cast_nullable_to_non_nullable
                    as Uint8List,
        inReplyTo:
            freezed == inReplyTo
                ? _value.inReplyTo
                : inReplyTo // ignore: cast_nullable_to_non_nullable
                    as Uint8List?,
        plainBody:
            null == plainBody
                ? _value.plainBody
                : plainBody // ignore: cast_nullable_to_non_nullable
                    as String,
        content:
            null == content
                ? _value.content
                : content // ignore: cast_nullable_to_non_nullable
                    as MessageContent,
      ),
    );
  }
}

/// @nodoc

class _$UiMimiContentImpl implements _UiMimiContent {
  const _$UiMimiContentImpl({
    this.replaces,
    required this.topicId,
    this.inReplyTo,
    required this.plainBody,
    required this.content,
  });

  @override
  final Uint8List? replaces;
  @override
  final Uint8List topicId;
  @override
  final Uint8List? inReplyTo;
  @override
  final String plainBody;
  @override
  final MessageContent content;

  @override
  String toString() {
    return 'UiMimiContent(replaces: $replaces, topicId: $topicId, inReplyTo: $inReplyTo, plainBody: $plainBody, content: $content)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiMimiContentImpl &&
            const DeepCollectionEquality().equals(other.replaces, replaces) &&
            const DeepCollectionEquality().equals(other.topicId, topicId) &&
            const DeepCollectionEquality().equals(other.inReplyTo, inReplyTo) &&
            (identical(other.plainBody, plainBody) ||
                other.plainBody == plainBody) &&
            (identical(other.content, content) || other.content == content));
  }

  @override
  int get hashCode => Object.hash(
    runtimeType,
    const DeepCollectionEquality().hash(replaces),
    const DeepCollectionEquality().hash(topicId),
    const DeepCollectionEquality().hash(inReplyTo),
    plainBody,
    content,
  );

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMimiContentImplCopyWith<_$UiMimiContentImpl> get copyWith =>
      __$$UiMimiContentImplCopyWithImpl<_$UiMimiContentImpl>(this, _$identity);
}

abstract class _UiMimiContent implements UiMimiContent {
  const factory _UiMimiContent({
    final Uint8List? replaces,
    required final Uint8List topicId,
    final Uint8List? inReplyTo,
    required final String plainBody,
    required final MessageContent content,
  }) = _$UiMimiContentImpl;

  @override
  Uint8List? get replaces;
  @override
  Uint8List get topicId;
  @override
  Uint8List? get inReplyTo;
  @override
  String get plainBody;
  @override
  MessageContent get content;

  /// Create a copy of UiMimiContent
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiMimiContentImplCopyWith<_$UiMimiContentImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UiUserHandle {
  String get plaintext => throw _privateConstructorUsedError;

  /// Create a copy of UiUserHandle
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $UiUserHandleCopyWith<UiUserHandle> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiUserHandleCopyWith<$Res> {
  factory $UiUserHandleCopyWith(
    UiUserHandle value,
    $Res Function(UiUserHandle) then,
  ) = _$UiUserHandleCopyWithImpl<$Res, UiUserHandle>;
  @useResult
  $Res call({String plaintext});
}

/// @nodoc
class _$UiUserHandleCopyWithImpl<$Res, $Val extends UiUserHandle>
    implements $UiUserHandleCopyWith<$Res> {
  _$UiUserHandleCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UiUserHandle
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? plaintext = null}) {
    return _then(
      _value.copyWith(
            plaintext:
                null == plaintext
                    ? _value.plaintext
                    : plaintext // ignore: cast_nullable_to_non_nullable
                        as String,
          )
          as $Val,
    );
  }
}

/// @nodoc
abstract class _$$UiUserHandleImplCopyWith<$Res>
    implements $UiUserHandleCopyWith<$Res> {
  factory _$$UiUserHandleImplCopyWith(
    _$UiUserHandleImpl value,
    $Res Function(_$UiUserHandleImpl) then,
  ) = __$$UiUserHandleImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({String plaintext});
}

/// @nodoc
class __$$UiUserHandleImplCopyWithImpl<$Res>
    extends _$UiUserHandleCopyWithImpl<$Res, _$UiUserHandleImpl>
    implements _$$UiUserHandleImplCopyWith<$Res> {
  __$$UiUserHandleImplCopyWithImpl(
    _$UiUserHandleImpl _value,
    $Res Function(_$UiUserHandleImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of UiUserHandle
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? plaintext = null}) {
    return _then(
      _$UiUserHandleImpl(
        plaintext:
            null == plaintext
                ? _value.plaintext
                : plaintext // ignore: cast_nullable_to_non_nullable
                    as String,
      ),
    );
  }
}

/// @nodoc

class _$UiUserHandleImpl extends _UiUserHandle {
  const _$UiUserHandleImpl({required this.plaintext}) : super._();

  @override
  final String plaintext;

  @override
  String toString() {
    return 'UiUserHandle(plaintext: $plaintext)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiUserHandleImpl &&
            (identical(other.plaintext, plaintext) ||
                other.plaintext == plaintext));
  }

  @override
  int get hashCode => Object.hash(runtimeType, plaintext);

  /// Create a copy of UiUserHandle
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UiUserHandleImplCopyWith<_$UiUserHandleImpl> get copyWith =>
      __$$UiUserHandleImplCopyWithImpl<_$UiUserHandleImpl>(this, _$identity);
}

abstract class _UiUserHandle extends UiUserHandle {
  const factory _UiUserHandle({required final String plaintext}) =
      _$UiUserHandleImpl;
  const _UiUserHandle._() : super._();

  @override
  String get plaintext;

  /// Create a copy of UiUserHandle
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UiUserHandleImplCopyWith<_$UiUserHandleImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
