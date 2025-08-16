// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'loadable_user_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$LoadableUser {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LoadableUser);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'LoadableUser()';
}


}

/// @nodoc
class $LoadableUserCopyWith<$Res>  {
$LoadableUserCopyWith(LoadableUser _, $Res Function(LoadableUser) __);
}



/// @nodoc


class LoadingUser extends LoadableUser {
  const LoadingUser(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LoadingUser);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'LoadableUser.loading()';
}


}




/// @nodoc


class LoadedUser extends LoadableUser {
  const LoadedUser(this.user): super._();
  

 final  User? user;

/// Create a copy of LoadableUser
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LoadedUserCopyWith<LoadedUser> get copyWith => _$LoadedUserCopyWithImpl<LoadedUser>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LoadedUser&&(identical(other.user, user) || other.user == user));
}


@override
int get hashCode => Object.hash(runtimeType,user);

@override
String toString() {
  return 'LoadableUser.loaded(user: $user)';
}


}

/// @nodoc
abstract mixin class $LoadedUserCopyWith<$Res> implements $LoadableUserCopyWith<$Res> {
  factory $LoadedUserCopyWith(LoadedUser value, $Res Function(LoadedUser) _then) = _$LoadedUserCopyWithImpl;
@useResult
$Res call({
 User? user
});




}
/// @nodoc
class _$LoadedUserCopyWithImpl<$Res>
    implements $LoadedUserCopyWith<$Res> {
  _$LoadedUserCopyWithImpl(this._self, this._then);

  final LoadedUser _self;
  final $Res Function(LoadedUser) _then;

/// Create a copy of LoadableUser
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? user = freezed,}) {
  return _then(LoadedUser(
freezed == user ? _self.user : user // ignore: cast_nullable_to_non_nullable
as User?,
  ));
}


}

// dart format on
