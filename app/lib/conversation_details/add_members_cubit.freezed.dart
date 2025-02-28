// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'add_members_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$AddMembersState {
  List<UiContact> get contacts => throw _privateConstructorUsedError;
  Set<String> get selectedContacts => throw _privateConstructorUsedError;

  /// Create a copy of AddMembersState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $AddMembersStateCopyWith<AddMembersState> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $AddMembersStateCopyWith<$Res> {
  factory $AddMembersStateCopyWith(
          AddMembersState value, $Res Function(AddMembersState) then) =
      _$AddMembersStateCopyWithImpl<$Res, AddMembersState>;
  @useResult
  $Res call({List<UiContact> contacts, Set<String> selectedContacts});
}

/// @nodoc
class _$AddMembersStateCopyWithImpl<$Res, $Val extends AddMembersState>
    implements $AddMembersStateCopyWith<$Res> {
  _$AddMembersStateCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of AddMembersState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? contacts = null,
    Object? selectedContacts = null,
  }) {
    return _then(_value.copyWith(
      contacts: null == contacts
          ? _value.contacts
          : contacts // ignore: cast_nullable_to_non_nullable
              as List<UiContact>,
      selectedContacts: null == selectedContacts
          ? _value.selectedContacts
          : selectedContacts // ignore: cast_nullable_to_non_nullable
              as Set<String>,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$AddMembersStateImplCopyWith<$Res>
    implements $AddMembersStateCopyWith<$Res> {
  factory _$$AddMembersStateImplCopyWith(_$AddMembersStateImpl value,
          $Res Function(_$AddMembersStateImpl) then) =
      __$$AddMembersStateImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({List<UiContact> contacts, Set<String> selectedContacts});
}

/// @nodoc
class __$$AddMembersStateImplCopyWithImpl<$Res>
    extends _$AddMembersStateCopyWithImpl<$Res, _$AddMembersStateImpl>
    implements _$$AddMembersStateImplCopyWith<$Res> {
  __$$AddMembersStateImplCopyWithImpl(
      _$AddMembersStateImpl _value, $Res Function(_$AddMembersStateImpl) _then)
      : super(_value, _then);

  /// Create a copy of AddMembersState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? contacts = null,
    Object? selectedContacts = null,
  }) {
    return _then(_$AddMembersStateImpl(
      contacts: null == contacts
          ? _value._contacts
          : contacts // ignore: cast_nullable_to_non_nullable
              as List<UiContact>,
      selectedContacts: null == selectedContacts
          ? _value._selectedContacts
          : selectedContacts // ignore: cast_nullable_to_non_nullable
              as Set<String>,
    ));
  }
}

/// @nodoc

class _$AddMembersStateImpl implements _AddMembersState {
  const _$AddMembersStateImpl(
      {required final List<UiContact> contacts,
      required final Set<String> selectedContacts})
      : _contacts = contacts,
        _selectedContacts = selectedContacts;

  final List<UiContact> _contacts;
  @override
  List<UiContact> get contacts {
    if (_contacts is EqualUnmodifiableListView) return _contacts;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_contacts);
  }

  final Set<String> _selectedContacts;
  @override
  Set<String> get selectedContacts {
    if (_selectedContacts is EqualUnmodifiableSetView) return _selectedContacts;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableSetView(_selectedContacts);
  }

  @override
  String toString() {
    return 'AddMembersState(contacts: $contacts, selectedContacts: $selectedContacts)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$AddMembersStateImpl &&
            const DeepCollectionEquality().equals(other._contacts, _contacts) &&
            const DeepCollectionEquality()
                .equals(other._selectedContacts, _selectedContacts));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType,
      const DeepCollectionEquality().hash(_contacts),
      const DeepCollectionEquality().hash(_selectedContacts));

  /// Create a copy of AddMembersState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$AddMembersStateImplCopyWith<_$AddMembersStateImpl> get copyWith =>
      __$$AddMembersStateImplCopyWithImpl<_$AddMembersStateImpl>(
          this, _$identity);
}

abstract class _AddMembersState implements AddMembersState {
  const factory _AddMembersState(
      {required final List<UiContact> contacts,
      required final Set<String> selectedContacts}) = _$AddMembersStateImpl;

  @override
  List<UiContact> get contacts;
  @override
  Set<String> get selectedContacts;

  /// Create a copy of AddMembersState
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$AddMembersStateImplCopyWith<_$AddMembersStateImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
