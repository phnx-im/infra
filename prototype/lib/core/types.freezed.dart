// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$UiConversationStatus {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiInactiveConversation field0) inactive,
    required TResult Function() active,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiInactiveConversation field0)? inactive,
    TResult? Function()? active,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiInactiveConversation field0)? inactive,
    TResult Function()? active,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationStatus_Inactive value) inactive,
    required TResult Function(UiConversationStatus_Active value) active,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationStatus_Inactive value)? inactive,
    TResult? Function(UiConversationStatus_Active value)? active,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationStatus_Inactive value)? inactive,
    TResult Function(UiConversationStatus_Active value)? active,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiConversationStatusCopyWith<$Res> {
  factory $UiConversationStatusCopyWith(UiConversationStatus value,
          $Res Function(UiConversationStatus) then) =
      _$UiConversationStatusCopyWithImpl<$Res, UiConversationStatus>;
}

/// @nodoc
class _$UiConversationStatusCopyWithImpl<$Res,
        $Val extends UiConversationStatus>
    implements $UiConversationStatusCopyWith<$Res> {
  _$UiConversationStatusCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$UiConversationStatus_InactiveImplCopyWith<$Res> {
  factory _$$UiConversationStatus_InactiveImplCopyWith(
          _$UiConversationStatus_InactiveImpl value,
          $Res Function(_$UiConversationStatus_InactiveImpl) then) =
      __$$UiConversationStatus_InactiveImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiInactiveConversation field0});
}

/// @nodoc
class __$$UiConversationStatus_InactiveImplCopyWithImpl<$Res>
    extends _$UiConversationStatusCopyWithImpl<$Res,
        _$UiConversationStatus_InactiveImpl>
    implements _$$UiConversationStatus_InactiveImplCopyWith<$Res> {
  __$$UiConversationStatus_InactiveImplCopyWithImpl(
      _$UiConversationStatus_InactiveImpl _value,
      $Res Function(_$UiConversationStatus_InactiveImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiConversationStatus_InactiveImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiInactiveConversation,
    ));
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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationStatus_InactiveImplCopyWith<
          _$UiConversationStatus_InactiveImpl>
      get copyWith => __$$UiConversationStatus_InactiveImplCopyWithImpl<
          _$UiConversationStatus_InactiveImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiInactiveConversation field0) inactive,
    required TResult Function() active,
  }) {
    return inactive(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiInactiveConversation field0)? inactive,
    TResult? Function()? active,
  }) {
    return inactive?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiInactiveConversation field0)? inactive,
    TResult Function()? active,
    required TResult orElse(),
  }) {
    if (inactive != null) {
      return inactive(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationStatus_Inactive value) inactive,
    required TResult Function(UiConversationStatus_Active value) active,
  }) {
    return inactive(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationStatus_Inactive value)? inactive,
    TResult? Function(UiConversationStatus_Active value)? active,
  }) {
    return inactive?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationStatus_Inactive value)? inactive,
    TResult Function(UiConversationStatus_Active value)? active,
    required TResult orElse(),
  }) {
    if (inactive != null) {
      return inactive(this);
    }
    return orElse();
  }
}

abstract class UiConversationStatus_Inactive extends UiConversationStatus {
  const factory UiConversationStatus_Inactive(
          final UiInactiveConversation field0) =
      _$UiConversationStatus_InactiveImpl;
  const UiConversationStatus_Inactive._() : super._();

  UiInactiveConversation get field0;
  @JsonKey(ignore: true)
  _$$UiConversationStatus_InactiveImplCopyWith<
          _$UiConversationStatus_InactiveImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationStatus_ActiveImplCopyWith<$Res> {
  factory _$$UiConversationStatus_ActiveImplCopyWith(
          _$UiConversationStatus_ActiveImpl value,
          $Res Function(_$UiConversationStatus_ActiveImpl) then) =
      __$$UiConversationStatus_ActiveImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$UiConversationStatus_ActiveImplCopyWithImpl<$Res>
    extends _$UiConversationStatusCopyWithImpl<$Res,
        _$UiConversationStatus_ActiveImpl>
    implements _$$UiConversationStatus_ActiveImplCopyWith<$Res> {
  __$$UiConversationStatus_ActiveImplCopyWithImpl(
      _$UiConversationStatus_ActiveImpl _value,
      $Res Function(_$UiConversationStatus_ActiveImpl) _then)
      : super(_value, _then);
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

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiInactiveConversation field0) inactive,
    required TResult Function() active,
  }) {
    return active();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiInactiveConversation field0)? inactive,
    TResult? Function()? active,
  }) {
    return active?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiInactiveConversation field0)? inactive,
    TResult Function()? active,
    required TResult orElse(),
  }) {
    if (active != null) {
      return active();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationStatus_Inactive value) inactive,
    required TResult Function(UiConversationStatus_Active value) active,
  }) {
    return active(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationStatus_Inactive value)? inactive,
    TResult? Function(UiConversationStatus_Active value)? active,
  }) {
    return active?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationStatus_Inactive value)? inactive,
    TResult Function(UiConversationStatus_Active value)? active,
    required TResult orElse(),
  }) {
    if (active != null) {
      return active(this);
    }
    return orElse();
  }
}

abstract class UiConversationStatus_Active extends UiConversationStatus {
  const factory UiConversationStatus_Active() =
      _$UiConversationStatus_ActiveImpl;
  const UiConversationStatus_Active._() : super._();
}

/// @nodoc
mixin _$UiConversationType {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) unconfirmedConnection,
    required TResult Function(String field0) connection,
    required TResult Function() group,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? unconfirmedConnection,
    TResult? Function(String field0)? connection,
    TResult? Function()? group,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? unconfirmedConnection,
    TResult Function(String field0)? connection,
    TResult Function()? group,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationType_UnconfirmedConnection value)
        unconfirmedConnection,
    required TResult Function(UiConversationType_Connection value) connection,
    required TResult Function(UiConversationType_Group value) group,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult? Function(UiConversationType_Connection value)? connection,
    TResult? Function(UiConversationType_Group value)? group,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult Function(UiConversationType_Connection value)? connection,
    TResult Function(UiConversationType_Group value)? group,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiConversationTypeCopyWith<$Res> {
  factory $UiConversationTypeCopyWith(
          UiConversationType value, $Res Function(UiConversationType) then) =
      _$UiConversationTypeCopyWithImpl<$Res, UiConversationType>;
}

/// @nodoc
class _$UiConversationTypeCopyWithImpl<$Res, $Val extends UiConversationType>
    implements $UiConversationTypeCopyWith<$Res> {
  _$UiConversationTypeCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$UiConversationType_UnconfirmedConnectionImplCopyWith<$Res> {
  factory _$$UiConversationType_UnconfirmedConnectionImplCopyWith(
          _$UiConversationType_UnconfirmedConnectionImpl value,
          $Res Function(_$UiConversationType_UnconfirmedConnectionImpl) then) =
      __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<$Res>
    extends _$UiConversationTypeCopyWithImpl<$Res,
        _$UiConversationType_UnconfirmedConnectionImpl>
    implements _$$UiConversationType_UnconfirmedConnectionImplCopyWith<$Res> {
  __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl(
      _$UiConversationType_UnconfirmedConnectionImpl _value,
      $Res Function(_$UiConversationType_UnconfirmedConnectionImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiConversationType_UnconfirmedConnectionImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$UiConversationType_UnconfirmedConnectionImpl
    extends UiConversationType_UnconfirmedConnection {
  const _$UiConversationType_UnconfirmedConnectionImpl(this.field0) : super._();

  @override
  final String field0;

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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationType_UnconfirmedConnectionImplCopyWith<
          _$UiConversationType_UnconfirmedConnectionImpl>
      get copyWith =>
          __$$UiConversationType_UnconfirmedConnectionImplCopyWithImpl<
              _$UiConversationType_UnconfirmedConnectionImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) unconfirmedConnection,
    required TResult Function(String field0) connection,
    required TResult Function() group,
  }) {
    return unconfirmedConnection(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? unconfirmedConnection,
    TResult? Function(String field0)? connection,
    TResult? Function()? group,
  }) {
    return unconfirmedConnection?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? unconfirmedConnection,
    TResult Function(String field0)? connection,
    TResult Function()? group,
    required TResult orElse(),
  }) {
    if (unconfirmedConnection != null) {
      return unconfirmedConnection(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationType_UnconfirmedConnection value)
        unconfirmedConnection,
    required TResult Function(UiConversationType_Connection value) connection,
    required TResult Function(UiConversationType_Group value) group,
  }) {
    return unconfirmedConnection(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult? Function(UiConversationType_Connection value)? connection,
    TResult? Function(UiConversationType_Group value)? group,
  }) {
    return unconfirmedConnection?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult Function(UiConversationType_Connection value)? connection,
    TResult Function(UiConversationType_Group value)? group,
    required TResult orElse(),
  }) {
    if (unconfirmedConnection != null) {
      return unconfirmedConnection(this);
    }
    return orElse();
  }
}

abstract class UiConversationType_UnconfirmedConnection
    extends UiConversationType {
  const factory UiConversationType_UnconfirmedConnection(final String field0) =
      _$UiConversationType_UnconfirmedConnectionImpl;
  const UiConversationType_UnconfirmedConnection._() : super._();

  String get field0;
  @JsonKey(ignore: true)
  _$$UiConversationType_UnconfirmedConnectionImplCopyWith<
          _$UiConversationType_UnconfirmedConnectionImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationType_ConnectionImplCopyWith<$Res> {
  factory _$$UiConversationType_ConnectionImplCopyWith(
          _$UiConversationType_ConnectionImpl value,
          $Res Function(_$UiConversationType_ConnectionImpl) then) =
      __$$UiConversationType_ConnectionImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$UiConversationType_ConnectionImplCopyWithImpl<$Res>
    extends _$UiConversationTypeCopyWithImpl<$Res,
        _$UiConversationType_ConnectionImpl>
    implements _$$UiConversationType_ConnectionImplCopyWith<$Res> {
  __$$UiConversationType_ConnectionImplCopyWithImpl(
      _$UiConversationType_ConnectionImpl _value,
      $Res Function(_$UiConversationType_ConnectionImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiConversationType_ConnectionImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$UiConversationType_ConnectionImpl
    extends UiConversationType_Connection {
  const _$UiConversationType_ConnectionImpl(this.field0) : super._();

  @override
  final String field0;

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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiConversationType_ConnectionImplCopyWith<
          _$UiConversationType_ConnectionImpl>
      get copyWith => __$$UiConversationType_ConnectionImplCopyWithImpl<
          _$UiConversationType_ConnectionImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) unconfirmedConnection,
    required TResult Function(String field0) connection,
    required TResult Function() group,
  }) {
    return connection(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? unconfirmedConnection,
    TResult? Function(String field0)? connection,
    TResult? Function()? group,
  }) {
    return connection?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? unconfirmedConnection,
    TResult Function(String field0)? connection,
    TResult Function()? group,
    required TResult orElse(),
  }) {
    if (connection != null) {
      return connection(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationType_UnconfirmedConnection value)
        unconfirmedConnection,
    required TResult Function(UiConversationType_Connection value) connection,
    required TResult Function(UiConversationType_Group value) group,
  }) {
    return connection(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult? Function(UiConversationType_Connection value)? connection,
    TResult? Function(UiConversationType_Group value)? group,
  }) {
    return connection?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult Function(UiConversationType_Connection value)? connection,
    TResult Function(UiConversationType_Group value)? group,
    required TResult orElse(),
  }) {
    if (connection != null) {
      return connection(this);
    }
    return orElse();
  }
}

abstract class UiConversationType_Connection extends UiConversationType {
  const factory UiConversationType_Connection(final String field0) =
      _$UiConversationType_ConnectionImpl;
  const UiConversationType_Connection._() : super._();

  String get field0;
  @JsonKey(ignore: true)
  _$$UiConversationType_ConnectionImplCopyWith<
          _$UiConversationType_ConnectionImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiConversationType_GroupImplCopyWith<$Res> {
  factory _$$UiConversationType_GroupImplCopyWith(
          _$UiConversationType_GroupImpl value,
          $Res Function(_$UiConversationType_GroupImpl) then) =
      __$$UiConversationType_GroupImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$UiConversationType_GroupImplCopyWithImpl<$Res>
    extends _$UiConversationTypeCopyWithImpl<$Res,
        _$UiConversationType_GroupImpl>
    implements _$$UiConversationType_GroupImplCopyWith<$Res> {
  __$$UiConversationType_GroupImplCopyWithImpl(
      _$UiConversationType_GroupImpl _value,
      $Res Function(_$UiConversationType_GroupImpl) _then)
      : super(_value, _then);
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

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) unconfirmedConnection,
    required TResult Function(String field0) connection,
    required TResult Function() group,
  }) {
    return group();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? unconfirmedConnection,
    TResult? Function(String field0)? connection,
    TResult? Function()? group,
  }) {
    return group?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? unconfirmedConnection,
    TResult Function(String field0)? connection,
    TResult Function()? group,
    required TResult orElse(),
  }) {
    if (group != null) {
      return group();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiConversationType_UnconfirmedConnection value)
        unconfirmedConnection,
    required TResult Function(UiConversationType_Connection value) connection,
    required TResult Function(UiConversationType_Group value) group,
  }) {
    return group(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult? Function(UiConversationType_Connection value)? connection,
    TResult? Function(UiConversationType_Group value)? group,
  }) {
    return group?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiConversationType_UnconfirmedConnection value)?
        unconfirmedConnection,
    TResult Function(UiConversationType_Connection value)? connection,
    TResult Function(UiConversationType_Group value)? group,
    required TResult orElse(),
  }) {
    if (group != null) {
      return group(this);
    }
    return orElse();
  }
}

abstract class UiConversationType_Group extends UiConversationType {
  const factory UiConversationType_Group() = _$UiConversationType_GroupImpl;
  const UiConversationType_Group._() : super._();
}

/// @nodoc
mixin _$UiEventMessage {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiSystemMessage field0) system,
    required TResult Function(UiErrorMessage field0) error,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiSystemMessage field0)? system,
    TResult? Function(UiErrorMessage field0)? error,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiSystemMessage field0)? system,
    TResult Function(UiErrorMessage field0)? error,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiEventMessage_System value) system,
    required TResult Function(UiEventMessage_Error value) error,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiEventMessage_System value)? system,
    TResult? Function(UiEventMessage_Error value)? error,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiEventMessage_System value)? system,
    TResult Function(UiEventMessage_Error value)? error,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiEventMessageCopyWith<$Res> {
  factory $UiEventMessageCopyWith(
          UiEventMessage value, $Res Function(UiEventMessage) then) =
      _$UiEventMessageCopyWithImpl<$Res, UiEventMessage>;
}

/// @nodoc
class _$UiEventMessageCopyWithImpl<$Res, $Val extends UiEventMessage>
    implements $UiEventMessageCopyWith<$Res> {
  _$UiEventMessageCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$UiEventMessage_SystemImplCopyWith<$Res> {
  factory _$$UiEventMessage_SystemImplCopyWith(
          _$UiEventMessage_SystemImpl value,
          $Res Function(_$UiEventMessage_SystemImpl) then) =
      __$$UiEventMessage_SystemImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiSystemMessage field0});
}

/// @nodoc
class __$$UiEventMessage_SystemImplCopyWithImpl<$Res>
    extends _$UiEventMessageCopyWithImpl<$Res, _$UiEventMessage_SystemImpl>
    implements _$$UiEventMessage_SystemImplCopyWith<$Res> {
  __$$UiEventMessage_SystemImplCopyWithImpl(_$UiEventMessage_SystemImpl _value,
      $Res Function(_$UiEventMessage_SystemImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiEventMessage_SystemImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiSystemMessage,
    ));
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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiEventMessage_SystemImplCopyWith<_$UiEventMessage_SystemImpl>
      get copyWith => __$$UiEventMessage_SystemImplCopyWithImpl<
          _$UiEventMessage_SystemImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiSystemMessage field0) system,
    required TResult Function(UiErrorMessage field0) error,
  }) {
    return system(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiSystemMessage field0)? system,
    TResult? Function(UiErrorMessage field0)? error,
  }) {
    return system?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiSystemMessage field0)? system,
    TResult Function(UiErrorMessage field0)? error,
    required TResult orElse(),
  }) {
    if (system != null) {
      return system(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiEventMessage_System value) system,
    required TResult Function(UiEventMessage_Error value) error,
  }) {
    return system(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiEventMessage_System value)? system,
    TResult? Function(UiEventMessage_Error value)? error,
  }) {
    return system?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiEventMessage_System value)? system,
    TResult Function(UiEventMessage_Error value)? error,
    required TResult orElse(),
  }) {
    if (system != null) {
      return system(this);
    }
    return orElse();
  }
}

abstract class UiEventMessage_System extends UiEventMessage {
  const factory UiEventMessage_System(final UiSystemMessage field0) =
      _$UiEventMessage_SystemImpl;
  const UiEventMessage_System._() : super._();

  @override
  UiSystemMessage get field0;
  @JsonKey(ignore: true)
  _$$UiEventMessage_SystemImplCopyWith<_$UiEventMessage_SystemImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiEventMessage_ErrorImplCopyWith<$Res> {
  factory _$$UiEventMessage_ErrorImplCopyWith(_$UiEventMessage_ErrorImpl value,
          $Res Function(_$UiEventMessage_ErrorImpl) then) =
      __$$UiEventMessage_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiErrorMessage field0});
}

/// @nodoc
class __$$UiEventMessage_ErrorImplCopyWithImpl<$Res>
    extends _$UiEventMessageCopyWithImpl<$Res, _$UiEventMessage_ErrorImpl>
    implements _$$UiEventMessage_ErrorImplCopyWith<$Res> {
  __$$UiEventMessage_ErrorImplCopyWithImpl(_$UiEventMessage_ErrorImpl _value,
      $Res Function(_$UiEventMessage_ErrorImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiEventMessage_ErrorImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiErrorMessage,
    ));
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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiEventMessage_ErrorImplCopyWith<_$UiEventMessage_ErrorImpl>
      get copyWith =>
          __$$UiEventMessage_ErrorImplCopyWithImpl<_$UiEventMessage_ErrorImpl>(
              this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiSystemMessage field0) system,
    required TResult Function(UiErrorMessage field0) error,
  }) {
    return error(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiSystemMessage field0)? system,
    TResult? Function(UiErrorMessage field0)? error,
  }) {
    return error?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiSystemMessage field0)? system,
    TResult Function(UiErrorMessage field0)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiEventMessage_System value) system,
    required TResult Function(UiEventMessage_Error value) error,
  }) {
    return error(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiEventMessage_System value)? system,
    TResult? Function(UiEventMessage_Error value)? error,
  }) {
    return error?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiEventMessage_System value)? system,
    TResult Function(UiEventMessage_Error value)? error,
    required TResult orElse(),
  }) {
    if (error != null) {
      return error(this);
    }
    return orElse();
  }
}

abstract class UiEventMessage_Error extends UiEventMessage {
  const factory UiEventMessage_Error(final UiErrorMessage field0) =
      _$UiEventMessage_ErrorImpl;
  const UiEventMessage_Error._() : super._();

  @override
  UiErrorMessage get field0;
  @JsonKey(ignore: true)
  _$$UiEventMessage_ErrorImplCopyWith<_$UiEventMessage_ErrorImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UiMessage {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiContentMessage field0) content,
    required TResult Function(UiEventMessage field0) display,
    required TResult Function(UiMimiContent field0) unsent,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiContentMessage field0)? content,
    TResult? Function(UiEventMessage field0)? display,
    TResult? Function(UiMimiContent field0)? unsent,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiContentMessage field0)? content,
    TResult Function(UiEventMessage field0)? display,
    TResult Function(UiMimiContent field0)? unsent,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiMessage_Content value) content,
    required TResult Function(UiMessage_Display value) display,
    required TResult Function(UiMessage_Unsent value) unsent,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiMessage_Content value)? content,
    TResult? Function(UiMessage_Display value)? display,
    TResult? Function(UiMessage_Unsent value)? unsent,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiMessage_Content value)? content,
    TResult Function(UiMessage_Display value)? display,
    TResult Function(UiMessage_Unsent value)? unsent,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
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
}

/// @nodoc
abstract class _$$UiMessage_ContentImplCopyWith<$Res> {
  factory _$$UiMessage_ContentImplCopyWith(_$UiMessage_ContentImpl value,
          $Res Function(_$UiMessage_ContentImpl) then) =
      __$$UiMessage_ContentImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiContentMessage field0});
}

/// @nodoc
class __$$UiMessage_ContentImplCopyWithImpl<$Res>
    extends _$UiMessageCopyWithImpl<$Res, _$UiMessage_ContentImpl>
    implements _$$UiMessage_ContentImplCopyWith<$Res> {
  __$$UiMessage_ContentImplCopyWithImpl(_$UiMessage_ContentImpl _value,
      $Res Function(_$UiMessage_ContentImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiMessage_ContentImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiContentMessage,
    ));
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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMessage_ContentImplCopyWith<_$UiMessage_ContentImpl> get copyWith =>
      __$$UiMessage_ContentImplCopyWithImpl<_$UiMessage_ContentImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiContentMessage field0) content,
    required TResult Function(UiEventMessage field0) display,
    required TResult Function(UiMimiContent field0) unsent,
  }) {
    return content(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiContentMessage field0)? content,
    TResult? Function(UiEventMessage field0)? display,
    TResult? Function(UiMimiContent field0)? unsent,
  }) {
    return content?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiContentMessage field0)? content,
    TResult Function(UiEventMessage field0)? display,
    TResult Function(UiMimiContent field0)? unsent,
    required TResult orElse(),
  }) {
    if (content != null) {
      return content(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiMessage_Content value) content,
    required TResult Function(UiMessage_Display value) display,
    required TResult Function(UiMessage_Unsent value) unsent,
  }) {
    return content(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiMessage_Content value)? content,
    TResult? Function(UiMessage_Display value)? display,
    TResult? Function(UiMessage_Unsent value)? unsent,
  }) {
    return content?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiMessage_Content value)? content,
    TResult Function(UiMessage_Display value)? display,
    TResult Function(UiMessage_Unsent value)? unsent,
    required TResult orElse(),
  }) {
    if (content != null) {
      return content(this);
    }
    return orElse();
  }
}

abstract class UiMessage_Content extends UiMessage {
  const factory UiMessage_Content(final UiContentMessage field0) =
      _$UiMessage_ContentImpl;
  const UiMessage_Content._() : super._();

  @override
  UiContentMessage get field0;
  @JsonKey(ignore: true)
  _$$UiMessage_ContentImplCopyWith<_$UiMessage_ContentImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiMessage_DisplayImplCopyWith<$Res> {
  factory _$$UiMessage_DisplayImplCopyWith(_$UiMessage_DisplayImpl value,
          $Res Function(_$UiMessage_DisplayImpl) then) =
      __$$UiMessage_DisplayImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiEventMessage field0});

  $UiEventMessageCopyWith<$Res> get field0;
}

/// @nodoc
class __$$UiMessage_DisplayImplCopyWithImpl<$Res>
    extends _$UiMessageCopyWithImpl<$Res, _$UiMessage_DisplayImpl>
    implements _$$UiMessage_DisplayImplCopyWith<$Res> {
  __$$UiMessage_DisplayImplCopyWithImpl(_$UiMessage_DisplayImpl _value,
      $Res Function(_$UiMessage_DisplayImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiMessage_DisplayImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiEventMessage,
    ));
  }

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

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMessage_DisplayImplCopyWith<_$UiMessage_DisplayImpl> get copyWith =>
      __$$UiMessage_DisplayImplCopyWithImpl<_$UiMessage_DisplayImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiContentMessage field0) content,
    required TResult Function(UiEventMessage field0) display,
    required TResult Function(UiMimiContent field0) unsent,
  }) {
    return display(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiContentMessage field0)? content,
    TResult? Function(UiEventMessage field0)? display,
    TResult? Function(UiMimiContent field0)? unsent,
  }) {
    return display?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiContentMessage field0)? content,
    TResult Function(UiEventMessage field0)? display,
    TResult Function(UiMimiContent field0)? unsent,
    required TResult orElse(),
  }) {
    if (display != null) {
      return display(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiMessage_Content value) content,
    required TResult Function(UiMessage_Display value) display,
    required TResult Function(UiMessage_Unsent value) unsent,
  }) {
    return display(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiMessage_Content value)? content,
    TResult? Function(UiMessage_Display value)? display,
    TResult? Function(UiMessage_Unsent value)? unsent,
  }) {
    return display?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiMessage_Content value)? content,
    TResult Function(UiMessage_Display value)? display,
    TResult Function(UiMessage_Unsent value)? unsent,
    required TResult orElse(),
  }) {
    if (display != null) {
      return display(this);
    }
    return orElse();
  }
}

abstract class UiMessage_Display extends UiMessage {
  const factory UiMessage_Display(final UiEventMessage field0) =
      _$UiMessage_DisplayImpl;
  const UiMessage_Display._() : super._();

  @override
  UiEventMessage get field0;
  @JsonKey(ignore: true)
  _$$UiMessage_DisplayImplCopyWith<_$UiMessage_DisplayImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiMessage_UnsentImplCopyWith<$Res> {
  factory _$$UiMessage_UnsentImplCopyWith(_$UiMessage_UnsentImpl value,
          $Res Function(_$UiMessage_UnsentImpl) then) =
      __$$UiMessage_UnsentImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiMimiContent field0});
}

/// @nodoc
class __$$UiMessage_UnsentImplCopyWithImpl<$Res>
    extends _$UiMessageCopyWithImpl<$Res, _$UiMessage_UnsentImpl>
    implements _$$UiMessage_UnsentImplCopyWith<$Res> {
  __$$UiMessage_UnsentImplCopyWithImpl(_$UiMessage_UnsentImpl _value,
      $Res Function(_$UiMessage_UnsentImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiMessage_UnsentImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiMimiContent,
    ));
  }
}

/// @nodoc

class _$UiMessage_UnsentImpl extends UiMessage_Unsent {
  const _$UiMessage_UnsentImpl(this.field0) : super._();

  @override
  final UiMimiContent field0;

  @override
  String toString() {
    return 'UiMessage.unsent(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiMessage_UnsentImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiMessage_UnsentImplCopyWith<_$UiMessage_UnsentImpl> get copyWith =>
      __$$UiMessage_UnsentImplCopyWithImpl<_$UiMessage_UnsentImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(UiContentMessage field0) content,
    required TResult Function(UiEventMessage field0) display,
    required TResult Function(UiMimiContent field0) unsent,
  }) {
    return unsent(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(UiContentMessage field0)? content,
    TResult? Function(UiEventMessage field0)? display,
    TResult? Function(UiMimiContent field0)? unsent,
  }) {
    return unsent?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(UiContentMessage field0)? content,
    TResult Function(UiEventMessage field0)? display,
    TResult Function(UiMimiContent field0)? unsent,
    required TResult orElse(),
  }) {
    if (unsent != null) {
      return unsent(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiMessage_Content value) content,
    required TResult Function(UiMessage_Display value) display,
    required TResult Function(UiMessage_Unsent value) unsent,
  }) {
    return unsent(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiMessage_Content value)? content,
    TResult? Function(UiMessage_Display value)? display,
    TResult? Function(UiMessage_Unsent value)? unsent,
  }) {
    return unsent?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiMessage_Content value)? content,
    TResult Function(UiMessage_Display value)? display,
    TResult Function(UiMessage_Unsent value)? unsent,
    required TResult orElse(),
  }) {
    if (unsent != null) {
      return unsent(this);
    }
    return orElse();
  }
}

abstract class UiMessage_Unsent extends UiMessage {
  const factory UiMessage_Unsent(final UiMimiContent field0) =
      _$UiMessage_UnsentImpl;
  const UiMessage_Unsent._() : super._();

  @override
  UiMimiContent get field0;
  @JsonKey(ignore: true)
  _$$UiMessage_UnsentImplCopyWith<_$UiMessage_UnsentImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UiNotificationType {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ConversationIdBytes field0) conversationChange,
    required TResult Function(UiConversationMessage field0) message,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ConversationIdBytes field0)? conversationChange,
    TResult? Function(UiConversationMessage field0)? message,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ConversationIdBytes field0)? conversationChange,
    TResult Function(UiConversationMessage field0)? message,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiNotificationType_ConversationChange value)
        conversationChange,
    required TResult Function(UiNotificationType_Message value) message,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult? Function(UiNotificationType_Message value)? message,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult Function(UiNotificationType_Message value)? message,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UiNotificationTypeCopyWith<$Res> {
  factory $UiNotificationTypeCopyWith(
          UiNotificationType value, $Res Function(UiNotificationType) then) =
      _$UiNotificationTypeCopyWithImpl<$Res, UiNotificationType>;
}

/// @nodoc
class _$UiNotificationTypeCopyWithImpl<$Res, $Val extends UiNotificationType>
    implements $UiNotificationTypeCopyWith<$Res> {
  _$UiNotificationTypeCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$UiNotificationType_ConversationChangeImplCopyWith<$Res> {
  factory _$$UiNotificationType_ConversationChangeImplCopyWith(
          _$UiNotificationType_ConversationChangeImpl value,
          $Res Function(_$UiNotificationType_ConversationChangeImpl) then) =
      __$$UiNotificationType_ConversationChangeImplCopyWithImpl<$Res>;
  @useResult
  $Res call({ConversationIdBytes field0});
}

/// @nodoc
class __$$UiNotificationType_ConversationChangeImplCopyWithImpl<$Res>
    extends _$UiNotificationTypeCopyWithImpl<$Res,
        _$UiNotificationType_ConversationChangeImpl>
    implements _$$UiNotificationType_ConversationChangeImplCopyWith<$Res> {
  __$$UiNotificationType_ConversationChangeImplCopyWithImpl(
      _$UiNotificationType_ConversationChangeImpl _value,
      $Res Function(_$UiNotificationType_ConversationChangeImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiNotificationType_ConversationChangeImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ConversationIdBytes,
    ));
  }
}

/// @nodoc

class _$UiNotificationType_ConversationChangeImpl
    extends UiNotificationType_ConversationChange {
  const _$UiNotificationType_ConversationChangeImpl(this.field0) : super._();

  @override
  final ConversationIdBytes field0;

  @override
  String toString() {
    return 'UiNotificationType.conversationChange(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiNotificationType_ConversationChangeImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiNotificationType_ConversationChangeImplCopyWith<
          _$UiNotificationType_ConversationChangeImpl>
      get copyWith => __$$UiNotificationType_ConversationChangeImplCopyWithImpl<
          _$UiNotificationType_ConversationChangeImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ConversationIdBytes field0) conversationChange,
    required TResult Function(UiConversationMessage field0) message,
  }) {
    return conversationChange(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ConversationIdBytes field0)? conversationChange,
    TResult? Function(UiConversationMessage field0)? message,
  }) {
    return conversationChange?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ConversationIdBytes field0)? conversationChange,
    TResult Function(UiConversationMessage field0)? message,
    required TResult orElse(),
  }) {
    if (conversationChange != null) {
      return conversationChange(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiNotificationType_ConversationChange value)
        conversationChange,
    required TResult Function(UiNotificationType_Message value) message,
  }) {
    return conversationChange(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult? Function(UiNotificationType_Message value)? message,
  }) {
    return conversationChange?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult Function(UiNotificationType_Message value)? message,
    required TResult orElse(),
  }) {
    if (conversationChange != null) {
      return conversationChange(this);
    }
    return orElse();
  }
}

abstract class UiNotificationType_ConversationChange
    extends UiNotificationType {
  const factory UiNotificationType_ConversationChange(
          final ConversationIdBytes field0) =
      _$UiNotificationType_ConversationChangeImpl;
  const UiNotificationType_ConversationChange._() : super._();

  @override
  ConversationIdBytes get field0;
  @JsonKey(ignore: true)
  _$$UiNotificationType_ConversationChangeImplCopyWith<
          _$UiNotificationType_ConversationChangeImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UiNotificationType_MessageImplCopyWith<$Res> {
  factory _$$UiNotificationType_MessageImplCopyWith(
          _$UiNotificationType_MessageImpl value,
          $Res Function(_$UiNotificationType_MessageImpl) then) =
      __$$UiNotificationType_MessageImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UiConversationMessage field0});
}

/// @nodoc
class __$$UiNotificationType_MessageImplCopyWithImpl<$Res>
    extends _$UiNotificationTypeCopyWithImpl<$Res,
        _$UiNotificationType_MessageImpl>
    implements _$$UiNotificationType_MessageImplCopyWith<$Res> {
  __$$UiNotificationType_MessageImplCopyWithImpl(
      _$UiNotificationType_MessageImpl _value,
      $Res Function(_$UiNotificationType_MessageImpl) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UiNotificationType_MessageImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UiConversationMessage,
    ));
  }
}

/// @nodoc

class _$UiNotificationType_MessageImpl extends UiNotificationType_Message {
  const _$UiNotificationType_MessageImpl(this.field0) : super._();

  @override
  final UiConversationMessage field0;

  @override
  String toString() {
    return 'UiNotificationType.message(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UiNotificationType_MessageImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$UiNotificationType_MessageImplCopyWith<_$UiNotificationType_MessageImpl>
      get copyWith => __$$UiNotificationType_MessageImplCopyWithImpl<
          _$UiNotificationType_MessageImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ConversationIdBytes field0) conversationChange,
    required TResult Function(UiConversationMessage field0) message,
  }) {
    return message(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ConversationIdBytes field0)? conversationChange,
    TResult? Function(UiConversationMessage field0)? message,
  }) {
    return message?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ConversationIdBytes field0)? conversationChange,
    TResult Function(UiConversationMessage field0)? message,
    required TResult orElse(),
  }) {
    if (message != null) {
      return message(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UiNotificationType_ConversationChange value)
        conversationChange,
    required TResult Function(UiNotificationType_Message value) message,
  }) {
    return message(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult? Function(UiNotificationType_Message value)? message,
  }) {
    return message?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UiNotificationType_ConversationChange value)?
        conversationChange,
    TResult Function(UiNotificationType_Message value)? message,
    required TResult orElse(),
  }) {
    if (message != null) {
      return message(this);
    }
    return orElse();
  }
}

abstract class UiNotificationType_Message extends UiNotificationType {
  const factory UiNotificationType_Message(final UiConversationMessage field0) =
      _$UiNotificationType_MessageImpl;
  const UiNotificationType_Message._() : super._();

  @override
  UiConversationMessage get field0;
  @JsonKey(ignore: true)
  _$$UiNotificationType_MessageImplCopyWith<_$UiNotificationType_MessageImpl>
      get copyWith => throw _privateConstructorUsedError;
}
