// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'navigation_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$NavigationState {}

/// @nodoc
abstract class $NavigationStateCopyWith<$Res> {
  factory $NavigationStateCopyWith(
          NavigationState value, $Res Function(NavigationState) then) =
      _$NavigationStateCopyWithImpl<$Res, NavigationState>;
}

/// @nodoc
class _$NavigationStateCopyWithImpl<$Res, $Val extends NavigationState>
    implements $NavigationStateCopyWith<$Res> {
  _$NavigationStateCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$IntroNavigationImplCopyWith<$Res> {
  factory _$$IntroNavigationImplCopyWith(_$IntroNavigationImpl value,
          $Res Function(_$IntroNavigationImpl) then) =
      __$$IntroNavigationImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<IntroScreenType> screens});
}

/// @nodoc
class __$$IntroNavigationImplCopyWithImpl<$Res>
    extends _$NavigationStateCopyWithImpl<$Res, _$IntroNavigationImpl>
    implements _$$IntroNavigationImplCopyWith<$Res> {
  __$$IntroNavigationImplCopyWithImpl(
      _$IntroNavigationImpl _value, $Res Function(_$IntroNavigationImpl) _then)
      : super(_value, _then);

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? screens = null,
  }) {
    return _then(_$IntroNavigationImpl(
      screens: null == screens
          ? _value._screens
          : screens // ignore: cast_nullable_to_non_nullable
              as List<IntroScreenType>,
    ));
  }
}

/// @nodoc

class _$IntroNavigationImpl extends IntroNavigation {
  const _$IntroNavigationImpl(
      {final List<IntroScreenType> screens = const [IntroScreenType.intro]})
      : _screens = screens,
        super._();

  final List<IntroScreenType> _screens;
  @override
  @JsonKey()
  List<IntroScreenType> get screens {
    if (_screens is EqualUnmodifiableListView) return _screens;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_screens);
  }

  @override
  String toString() {
    return 'NavigationState.intro(screens: $screens)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$IntroNavigationImpl &&
            const DeepCollectionEquality().equals(other._screens, _screens));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_screens));

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$IntroNavigationImplCopyWith<_$IntroNavigationImpl> get copyWith =>
      __$$IntroNavigationImplCopyWithImpl<_$IntroNavigationImpl>(
          this, _$identity);
}

abstract class IntroNavigation extends NavigationState {
  const factory IntroNavigation({final List<IntroScreenType> screens}) =
      _$IntroNavigationImpl;
  const IntroNavigation._() : super._();

  List<IntroScreenType> get screens;

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$IntroNavigationImplCopyWith<_$IntroNavigationImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$HomeNavigationImplCopyWith<$Res> {
  factory _$$HomeNavigationImplCopyWith(_$HomeNavigationImpl value,
          $Res Function(_$HomeNavigationImpl) then) =
      __$$HomeNavigationImplCopyWithImpl<$Res>;
  @useResult
  $Res call(
      {ConversationId? conversationId,
      DeveloperSettingsScreenType? developerSettingsScreen,
      bool userSettingsOpen,
      bool conversationDetailsOpen,
      bool addMembersOpen,
      String? memberDetails});
}

/// @nodoc
class __$$HomeNavigationImplCopyWithImpl<$Res>
    extends _$NavigationStateCopyWithImpl<$Res, _$HomeNavigationImpl>
    implements _$$HomeNavigationImplCopyWith<$Res> {
  __$$HomeNavigationImplCopyWithImpl(
      _$HomeNavigationImpl _value, $Res Function(_$HomeNavigationImpl) _then)
      : super(_value, _then);

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? conversationId = freezed,
    Object? developerSettingsScreen = freezed,
    Object? userSettingsOpen = null,
    Object? conversationDetailsOpen = null,
    Object? addMembersOpen = null,
    Object? memberDetails = freezed,
  }) {
    return _then(_$HomeNavigationImpl(
      conversationId: freezed == conversationId
          ? _value.conversationId
          : conversationId // ignore: cast_nullable_to_non_nullable
              as ConversationId?,
      developerSettingsScreen: freezed == developerSettingsScreen
          ? _value.developerSettingsScreen
          : developerSettingsScreen // ignore: cast_nullable_to_non_nullable
              as DeveloperSettingsScreenType?,
      userSettingsOpen: null == userSettingsOpen
          ? _value.userSettingsOpen
          : userSettingsOpen // ignore: cast_nullable_to_non_nullable
              as bool,
      conversationDetailsOpen: null == conversationDetailsOpen
          ? _value.conversationDetailsOpen
          : conversationDetailsOpen // ignore: cast_nullable_to_non_nullable
              as bool,
      addMembersOpen: null == addMembersOpen
          ? _value.addMembersOpen
          : addMembersOpen // ignore: cast_nullable_to_non_nullable
              as bool,
      memberDetails: freezed == memberDetails
          ? _value.memberDetails
          : memberDetails // ignore: cast_nullable_to_non_nullable
              as String?,
    ));
  }
}

/// @nodoc

class _$HomeNavigationImpl extends HomeNavigation {
  const _$HomeNavigationImpl(
      {this.conversationId,
      this.developerSettingsScreen,
      this.userSettingsOpen = false,
      this.conversationDetailsOpen = false,
      this.addMembersOpen = false,
      this.memberDetails})
      : super._();

  @override
  final ConversationId? conversationId;
  @override
  final DeveloperSettingsScreenType? developerSettingsScreen;
  @override
  @JsonKey()
  final bool userSettingsOpen;
  @override
  @JsonKey()
  final bool conversationDetailsOpen;
  @override
  @JsonKey()
  final bool addMembersOpen;

  /// User name of the member that details are currently open
  @override
  final String? memberDetails;

  @override
  String toString() {
    return 'NavigationState.home(conversationId: $conversationId, developerSettingsScreen: $developerSettingsScreen, userSettingsOpen: $userSettingsOpen, conversationDetailsOpen: $conversationDetailsOpen, addMembersOpen: $addMembersOpen, memberDetails: $memberDetails)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$HomeNavigationImpl &&
            (identical(other.conversationId, conversationId) ||
                other.conversationId == conversationId) &&
            (identical(
                    other.developerSettingsScreen, developerSettingsScreen) ||
                other.developerSettingsScreen == developerSettingsScreen) &&
            (identical(other.userSettingsOpen, userSettingsOpen) ||
                other.userSettingsOpen == userSettingsOpen) &&
            (identical(
                    other.conversationDetailsOpen, conversationDetailsOpen) ||
                other.conversationDetailsOpen == conversationDetailsOpen) &&
            (identical(other.addMembersOpen, addMembersOpen) ||
                other.addMembersOpen == addMembersOpen) &&
            (identical(other.memberDetails, memberDetails) ||
                other.memberDetails == memberDetails));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType,
      conversationId,
      developerSettingsScreen,
      userSettingsOpen,
      conversationDetailsOpen,
      addMembersOpen,
      memberDetails);

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$HomeNavigationImplCopyWith<_$HomeNavigationImpl> get copyWith =>
      __$$HomeNavigationImplCopyWithImpl<_$HomeNavigationImpl>(
          this, _$identity);
}

abstract class HomeNavigation extends NavigationState {
  const factory HomeNavigation(
      {final ConversationId? conversationId,
      final DeveloperSettingsScreenType? developerSettingsScreen,
      final bool userSettingsOpen,
      final bool conversationDetailsOpen,
      final bool addMembersOpen,
      final String? memberDetails}) = _$HomeNavigationImpl;
  const HomeNavigation._() : super._();

  ConversationId? get conversationId;
  DeveloperSettingsScreenType? get developerSettingsScreen;
  bool get userSettingsOpen;
  bool get conversationDetailsOpen;
  bool get addMembersOpen;

  /// User name of the member that details are currently open
  String? get memberDetails;

  /// Create a copy of NavigationState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$HomeNavigationImplCopyWith<_$HomeNavigationImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
