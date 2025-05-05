// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'registration_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$RegistrationState {
  // Domain choice screen data
  String get domain =>
      throw _privateConstructorUsedError; // Username screen data
  String get username => throw _privateConstructorUsedError;
  bool get isUsernameValid =>
      throw _privateConstructorUsedError; // Display name/avatar screen data
  ImageData? get avatar => throw _privateConstructorUsedError;
  String? get displayName => throw _privateConstructorUsedError;
  bool get isSigningUp => throw _privateConstructorUsedError;

  /// Create a copy of RegistrationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $RegistrationStateCopyWith<RegistrationState> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $RegistrationStateCopyWith<$Res> {
  factory $RegistrationStateCopyWith(
    RegistrationState value,
    $Res Function(RegistrationState) then,
  ) = _$RegistrationStateCopyWithImpl<$Res, RegistrationState>;
  @useResult
  $Res call({
    String domain,
    String username,
    bool isUsernameValid,
    ImageData? avatar,
    String? displayName,
    bool isSigningUp,
  });
}

/// @nodoc
class _$RegistrationStateCopyWithImpl<$Res, $Val extends RegistrationState>
    implements $RegistrationStateCopyWith<$Res> {
  _$RegistrationStateCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of RegistrationState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? domain = null,
    Object? username = null,
    Object? isUsernameValid = null,
    Object? avatar = freezed,
    Object? displayName = freezed,
    Object? isSigningUp = null,
  }) {
    return _then(
      _value.copyWith(
            domain:
                null == domain
                    ? _value.domain
                    : domain // ignore: cast_nullable_to_non_nullable
                        as String,
            username:
                null == username
                    ? _value.username
                    : username // ignore: cast_nullable_to_non_nullable
                        as String,
            isUsernameValid:
                null == isUsernameValid
                    ? _value.isUsernameValid
                    : isUsernameValid // ignore: cast_nullable_to_non_nullable
                        as bool,
            avatar:
                freezed == avatar
                    ? _value.avatar
                    : avatar // ignore: cast_nullable_to_non_nullable
                        as ImageData?,
            displayName:
                freezed == displayName
                    ? _value.displayName
                    : displayName // ignore: cast_nullable_to_non_nullable
                        as String?,
            isSigningUp:
                null == isSigningUp
                    ? _value.isSigningUp
                    : isSigningUp // ignore: cast_nullable_to_non_nullable
                        as bool,
          )
          as $Val,
    );
  }
}

/// @nodoc
abstract class _$$RegistrationStateImplCopyWith<$Res>
    implements $RegistrationStateCopyWith<$Res> {
  factory _$$RegistrationStateImplCopyWith(
    _$RegistrationStateImpl value,
    $Res Function(_$RegistrationStateImpl) then,
  ) = __$$RegistrationStateImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({
    String domain,
    String username,
    bool isUsernameValid,
    ImageData? avatar,
    String? displayName,
    bool isSigningUp,
  });
}

/// @nodoc
class __$$RegistrationStateImplCopyWithImpl<$Res>
    extends _$RegistrationStateCopyWithImpl<$Res, _$RegistrationStateImpl>
    implements _$$RegistrationStateImplCopyWith<$Res> {
  __$$RegistrationStateImplCopyWithImpl(
    _$RegistrationStateImpl _value,
    $Res Function(_$RegistrationStateImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of RegistrationState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? domain = null,
    Object? username = null,
    Object? isUsernameValid = null,
    Object? avatar = freezed,
    Object? displayName = freezed,
    Object? isSigningUp = null,
  }) {
    return _then(
      _$RegistrationStateImpl(
        domain:
            null == domain
                ? _value.domain
                : domain // ignore: cast_nullable_to_non_nullable
                    as String,
        username:
            null == username
                ? _value.username
                : username // ignore: cast_nullable_to_non_nullable
                    as String,
        isUsernameValid:
            null == isUsernameValid
                ? _value.isUsernameValid
                : isUsernameValid // ignore: cast_nullable_to_non_nullable
                    as bool,
        avatar:
            freezed == avatar
                ? _value.avatar
                : avatar // ignore: cast_nullable_to_non_nullable
                    as ImageData?,
        displayName:
            freezed == displayName
                ? _value.displayName
                : displayName // ignore: cast_nullable_to_non_nullable
                    as String?,
        isSigningUp:
            null == isSigningUp
                ? _value.isSigningUp
                : isSigningUp // ignore: cast_nullable_to_non_nullable
                    as bool,
      ),
    );
  }
}

/// @nodoc

class _$RegistrationStateImpl extends _RegistrationState {
  const _$RegistrationStateImpl({
    this.domain = '',
    this.username = '',
    this.isUsernameValid = false,
    this.avatar,
    this.displayName,
    this.isSigningUp = false,
  }) : super._();

  // Domain choice screen data
  @override
  @JsonKey()
  final String domain;
  // Username screen data
  @override
  @JsonKey()
  final String username;
  @override
  @JsonKey()
  final bool isUsernameValid;
  // Display name/avatar screen data
  @override
  final ImageData? avatar;
  @override
  final String? displayName;
  @override
  @JsonKey()
  final bool isSigningUp;

  @override
  String toString() {
    return 'RegistrationState(domain: $domain, username: $username, isUsernameValid: $isUsernameValid, avatar: $avatar, displayName: $displayName, isSigningUp: $isSigningUp)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RegistrationStateImpl &&
            (identical(other.domain, domain) || other.domain == domain) &&
            (identical(other.username, username) ||
                other.username == username) &&
            (identical(other.isUsernameValid, isUsernameValid) ||
                other.isUsernameValid == isUsernameValid) &&
            (identical(other.avatar, avatar) || other.avatar == avatar) &&
            (identical(other.displayName, displayName) ||
                other.displayName == displayName) &&
            (identical(other.isSigningUp, isSigningUp) ||
                other.isSigningUp == isSigningUp));
  }

  @override
  int get hashCode => Object.hash(
    runtimeType,
    domain,
    username,
    isUsernameValid,
    avatar,
    displayName,
    isSigningUp,
  );

  /// Create a copy of RegistrationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RegistrationStateImplCopyWith<_$RegistrationStateImpl> get copyWith =>
      __$$RegistrationStateImplCopyWithImpl<_$RegistrationStateImpl>(
        this,
        _$identity,
      );
}

abstract class _RegistrationState extends RegistrationState {
  const factory _RegistrationState({
    final String domain,
    final String username,
    final bool isUsernameValid,
    final ImageData? avatar,
    final String? displayName,
    final bool isSigningUp,
  }) = _$RegistrationStateImpl;
  const _RegistrationState._() : super._();

  // Domain choice screen data
  @override
  String get domain; // Username screen data
  @override
  String get username;
  @override
  bool get isUsernameValid; // Display name/avatar screen data
  @override
  ImageData? get avatar;
  @override
  String? get displayName;
  @override
  bool get isSigningUp;

  /// Create a copy of RegistrationState
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RegistrationStateImplCopyWith<_$RegistrationStateImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
