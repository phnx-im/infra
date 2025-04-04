// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'loadable_user_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$LoadableUser {}

/// @nodoc
abstract class $LoadableUserCopyWith<$Res> {
  factory $LoadableUserCopyWith(
    LoadableUser value,
    $Res Function(LoadableUser) then,
  ) = _$LoadableUserCopyWithImpl<$Res, LoadableUser>;
}

/// @nodoc
class _$LoadableUserCopyWithImpl<$Res, $Val extends LoadableUser>
    implements $LoadableUserCopyWith<$Res> {
  _$LoadableUserCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of LoadableUser
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$LoadingUserImplCopyWith<$Res> {
  factory _$$LoadingUserImplCopyWith(
    _$LoadingUserImpl value,
    $Res Function(_$LoadingUserImpl) then,
  ) = __$$LoadingUserImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$LoadingUserImplCopyWithImpl<$Res>
    extends _$LoadableUserCopyWithImpl<$Res, _$LoadingUserImpl>
    implements _$$LoadingUserImplCopyWith<$Res> {
  __$$LoadingUserImplCopyWithImpl(
    _$LoadingUserImpl _value,
    $Res Function(_$LoadingUserImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of LoadableUser
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$LoadingUserImpl extends LoadingUser {
  const _$LoadingUserImpl() : super._();

  @override
  String toString() {
    return 'LoadableUser.loading()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$LoadingUserImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;
}

abstract class LoadingUser extends LoadableUser {
  const factory LoadingUser() = _$LoadingUserImpl;
  const LoadingUser._() : super._();
}

/// @nodoc
abstract class _$$LoadedUserImplCopyWith<$Res> {
  factory _$$LoadedUserImplCopyWith(
    _$LoadedUserImpl value,
    $Res Function(_$LoadedUserImpl) then,
  ) = __$$LoadedUserImplCopyWithImpl<$Res>;
  @useResult
  $Res call({User? user});
}

/// @nodoc
class __$$LoadedUserImplCopyWithImpl<$Res>
    extends _$LoadableUserCopyWithImpl<$Res, _$LoadedUserImpl>
    implements _$$LoadedUserImplCopyWith<$Res> {
  __$$LoadedUserImplCopyWithImpl(
    _$LoadedUserImpl _value,
    $Res Function(_$LoadedUserImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of LoadableUser
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? user = freezed}) {
    return _then(
      _$LoadedUserImpl(
        freezed == user
            ? _value.user
            : user // ignore: cast_nullable_to_non_nullable
                as User?,
      ),
    );
  }
}

/// @nodoc

class _$LoadedUserImpl extends LoadedUser {
  const _$LoadedUserImpl(this.user) : super._();

  @override
  final User? user;

  @override
  String toString() {
    return 'LoadableUser.loaded(user: $user)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$LoadedUserImpl &&
            (identical(other.user, user) || other.user == user));
  }

  @override
  int get hashCode => Object.hash(runtimeType, user);

  /// Create a copy of LoadableUser
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$LoadedUserImplCopyWith<_$LoadedUserImpl> get copyWith =>
      __$$LoadedUserImplCopyWithImpl<_$LoadedUserImpl>(this, _$identity);
}

abstract class LoadedUser extends LoadableUser {
  const factory LoadedUser(final User? user) = _$LoadedUserImpl;
  const LoadedUser._() : super._();

  User? get user;

  /// Create a copy of LoadableUser
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$LoadedUserImplCopyWith<_$LoadedUserImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
