// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'user_settings_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UserSettings {

 double get interfaceScale; double get sidebarWidth;
/// Create a copy of UserSettings
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UserSettingsCopyWith<UserSettings> get copyWith => _$UserSettingsCopyWithImpl<UserSettings>(this as UserSettings, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UserSettings&&(identical(other.interfaceScale, interfaceScale) || other.interfaceScale == interfaceScale)&&(identical(other.sidebarWidth, sidebarWidth) || other.sidebarWidth == sidebarWidth));
}


@override
int get hashCode => Object.hash(runtimeType,interfaceScale,sidebarWidth);

@override
String toString() {
  return 'UserSettings(interfaceScale: $interfaceScale, sidebarWidth: $sidebarWidth)';
}


}

/// @nodoc
abstract mixin class $UserSettingsCopyWith<$Res>  {
  factory $UserSettingsCopyWith(UserSettings value, $Res Function(UserSettings) _then) = _$UserSettingsCopyWithImpl;
@useResult
$Res call({
 double interfaceScale, double sidebarWidth
});




}
/// @nodoc
class _$UserSettingsCopyWithImpl<$Res>
    implements $UserSettingsCopyWith<$Res> {
  _$UserSettingsCopyWithImpl(this._self, this._then);

  final UserSettings _self;
  final $Res Function(UserSettings) _then;

/// Create a copy of UserSettings
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? interfaceScale = null,Object? sidebarWidth = null,}) {
  return _then(_self.copyWith(
interfaceScale: null == interfaceScale ? _self.interfaceScale : interfaceScale // ignore: cast_nullable_to_non_nullable
as double,sidebarWidth: null == sidebarWidth ? _self.sidebarWidth : sidebarWidth // ignore: cast_nullable_to_non_nullable
as double,
  ));
}

}


/// @nodoc


class _UserSettings implements UserSettings {
  const _UserSettings({this.interfaceScale = 1.0, this.sidebarWidth = 300.0});
  

@override@JsonKey() final  double interfaceScale;
@override@JsonKey() final  double sidebarWidth;

/// Create a copy of UserSettings
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$UserSettingsCopyWith<_UserSettings> get copyWith => __$UserSettingsCopyWithImpl<_UserSettings>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _UserSettings&&(identical(other.interfaceScale, interfaceScale) || other.interfaceScale == interfaceScale)&&(identical(other.sidebarWidth, sidebarWidth) || other.sidebarWidth == sidebarWidth));
}


@override
int get hashCode => Object.hash(runtimeType,interfaceScale,sidebarWidth);

@override
String toString() {
  return 'UserSettings(interfaceScale: $interfaceScale, sidebarWidth: $sidebarWidth)';
}


}

/// @nodoc
abstract mixin class _$UserSettingsCopyWith<$Res> implements $UserSettingsCopyWith<$Res> {
  factory _$UserSettingsCopyWith(_UserSettings value, $Res Function(_UserSettings) _then) = __$UserSettingsCopyWithImpl;
@override @useResult
$Res call({
 double interfaceScale, double sidebarWidth
});




}
/// @nodoc
class __$UserSettingsCopyWithImpl<$Res>
    implements _$UserSettingsCopyWith<$Res> {
  __$UserSettingsCopyWithImpl(this._self, this._then);

  final _UserSettings _self;
  final $Res Function(_UserSettings) _then;

/// Create a copy of UserSettings
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? interfaceScale = null,Object? sidebarWidth = null,}) {
  return _then(_UserSettings(
interfaceScale: null == interfaceScale ? _self.interfaceScale : interfaceScale // ignore: cast_nullable_to_non_nullable
as double,sidebarWidth: null == sidebarWidth ? _self.sidebarWidth : sidebarWidth // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

// dart format on
