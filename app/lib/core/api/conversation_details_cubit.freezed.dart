// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'conversation_details_cubit.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$ConversationDetailsState {
  UiConversationDetails? get conversation => throw _privateConstructorUsedError;
  List<UiClientId> get members => throw _privateConstructorUsedError;

  /// Create a copy of ConversationDetailsState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $ConversationDetailsStateCopyWith<ConversationDetailsState> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ConversationDetailsStateCopyWith<$Res> {
  factory $ConversationDetailsStateCopyWith(
    ConversationDetailsState value,
    $Res Function(ConversationDetailsState) then,
  ) = _$ConversationDetailsStateCopyWithImpl<$Res, ConversationDetailsState>;
  @useResult
  $Res call({UiConversationDetails? conversation, List<UiClientId> members});
}

/// @nodoc
class _$ConversationDetailsStateCopyWithImpl<
  $Res,
  $Val extends ConversationDetailsState
>
    implements $ConversationDetailsStateCopyWith<$Res> {
  _$ConversationDetailsStateCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ConversationDetailsState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? conversation = freezed, Object? members = null}) {
    return _then(
      _value.copyWith(
            conversation:
                freezed == conversation
                    ? _value.conversation
                    : conversation // ignore: cast_nullable_to_non_nullable
                        as UiConversationDetails?,
            members:
                null == members
                    ? _value.members
                    : members // ignore: cast_nullable_to_non_nullable
                        as List<UiClientId>,
          )
          as $Val,
    );
  }
}

/// @nodoc
abstract class _$$ConversationDetailsStateImplCopyWith<$Res>
    implements $ConversationDetailsStateCopyWith<$Res> {
  factory _$$ConversationDetailsStateImplCopyWith(
    _$ConversationDetailsStateImpl value,
    $Res Function(_$ConversationDetailsStateImpl) then,
  ) = __$$ConversationDetailsStateImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({UiConversationDetails? conversation, List<UiClientId> members});
}

/// @nodoc
class __$$ConversationDetailsStateImplCopyWithImpl<$Res>
    extends
        _$ConversationDetailsStateCopyWithImpl<
          $Res,
          _$ConversationDetailsStateImpl
        >
    implements _$$ConversationDetailsStateImplCopyWith<$Res> {
  __$$ConversationDetailsStateImplCopyWithImpl(
    _$ConversationDetailsStateImpl _value,
    $Res Function(_$ConversationDetailsStateImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of ConversationDetailsState
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? conversation = freezed, Object? members = null}) {
    return _then(
      _$ConversationDetailsStateImpl(
        conversation:
            freezed == conversation
                ? _value.conversation
                : conversation // ignore: cast_nullable_to_non_nullable
                    as UiConversationDetails?,
        members:
            null == members
                ? _value._members
                : members // ignore: cast_nullable_to_non_nullable
                    as List<UiClientId>,
      ),
    );
  }
}

/// @nodoc

class _$ConversationDetailsStateImpl extends _ConversationDetailsState {
  const _$ConversationDetailsStateImpl({
    this.conversation,
    required final List<UiClientId> members,
  }) : _members = members,
       super._();

  @override
  final UiConversationDetails? conversation;
  final List<UiClientId> _members;
  @override
  List<UiClientId> get members {
    if (_members is EqualUnmodifiableListView) return _members;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_members);
  }

  @override
  String toString() {
    return 'ConversationDetailsState(conversation: $conversation, members: $members)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ConversationDetailsStateImpl &&
            (identical(other.conversation, conversation) ||
                other.conversation == conversation) &&
            const DeepCollectionEquality().equals(other._members, _members));
  }

  @override
  int get hashCode => Object.hash(
    runtimeType,
    conversation,
    const DeepCollectionEquality().hash(_members),
  );

  /// Create a copy of ConversationDetailsState
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ConversationDetailsStateImplCopyWith<_$ConversationDetailsStateImpl>
  get copyWith => __$$ConversationDetailsStateImplCopyWithImpl<
    _$ConversationDetailsStateImpl
  >(this, _$identity);
}

abstract class _ConversationDetailsState extends ConversationDetailsState {
  const factory _ConversationDetailsState({
    final UiConversationDetails? conversation,
    required final List<UiClientId> members,
  }) = _$ConversationDetailsStateImpl;
  const _ConversationDetailsState._() : super._();

  @override
  UiConversationDetails? get conversation;
  @override
  List<UiClientId> get members;

  /// Create a copy of ConversationDetailsState
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ConversationDetailsStateImplCopyWith<_$ConversationDetailsStateImpl>
  get copyWith => throw _privateConstructorUsedError;
}
