// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'markdown.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$BlockElement {}

/// @nodoc
abstract class $BlockElementCopyWith<$Res> {
  factory $BlockElementCopyWith(
          BlockElement value, $Res Function(BlockElement) then) =
      _$BlockElementCopyWithImpl<$Res, BlockElement>;
}

/// @nodoc
class _$BlockElementCopyWithImpl<$Res, $Val extends BlockElement>
    implements $BlockElementCopyWith<$Res> {
  _$BlockElementCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$BlockElement_ParagraphImplCopyWith<$Res> {
  factory _$$BlockElement_ParagraphImplCopyWith(
          _$BlockElement_ParagraphImpl value,
          $Res Function(_$BlockElement_ParagraphImpl) then) =
      __$$BlockElement_ParagraphImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$BlockElement_ParagraphImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_ParagraphImpl>
    implements _$$BlockElement_ParagraphImplCopyWith<$Res> {
  __$$BlockElement_ParagraphImplCopyWithImpl(
      _$BlockElement_ParagraphImpl _value,
      $Res Function(_$BlockElement_ParagraphImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_ParagraphImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$BlockElement_ParagraphImpl extends BlockElement_Paragraph {
  const _$BlockElement_ParagraphImpl(final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'BlockElement.paragraph(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_ParagraphImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_ParagraphImplCopyWith<_$BlockElement_ParagraphImpl>
      get copyWith => __$$BlockElement_ParagraphImplCopyWithImpl<
          _$BlockElement_ParagraphImpl>(this, _$identity);
}

abstract class BlockElement_Paragraph extends BlockElement {
  const factory BlockElement_Paragraph(final List<RangedInlineElement> field0) =
      _$BlockElement_ParagraphImpl;
  const BlockElement_Paragraph._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_ParagraphImplCopyWith<_$BlockElement_ParagraphImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_HeadingImplCopyWith<$Res> {
  factory _$$BlockElement_HeadingImplCopyWith(_$BlockElement_HeadingImpl value,
          $Res Function(_$BlockElement_HeadingImpl) then) =
      __$$BlockElement_HeadingImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$BlockElement_HeadingImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_HeadingImpl>
    implements _$$BlockElement_HeadingImplCopyWith<$Res> {
  __$$BlockElement_HeadingImplCopyWithImpl(_$BlockElement_HeadingImpl _value,
      $Res Function(_$BlockElement_HeadingImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_HeadingImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$BlockElement_HeadingImpl extends BlockElement_Heading {
  const _$BlockElement_HeadingImpl(final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'BlockElement.heading(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_HeadingImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_HeadingImplCopyWith<_$BlockElement_HeadingImpl>
      get copyWith =>
          __$$BlockElement_HeadingImplCopyWithImpl<_$BlockElement_HeadingImpl>(
              this, _$identity);
}

abstract class BlockElement_Heading extends BlockElement {
  const factory BlockElement_Heading(final List<RangedInlineElement> field0) =
      _$BlockElement_HeadingImpl;
  const BlockElement_Heading._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_HeadingImplCopyWith<_$BlockElement_HeadingImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_QuoteImplCopyWith<$Res> {
  factory _$$BlockElement_QuoteImplCopyWith(_$BlockElement_QuoteImpl value,
          $Res Function(_$BlockElement_QuoteImpl) then) =
      __$$BlockElement_QuoteImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedBlockElement> field0});
}

/// @nodoc
class __$$BlockElement_QuoteImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_QuoteImpl>
    implements _$$BlockElement_QuoteImplCopyWith<$Res> {
  __$$BlockElement_QuoteImplCopyWithImpl(_$BlockElement_QuoteImpl _value,
      $Res Function(_$BlockElement_QuoteImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_QuoteImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedBlockElement>,
    ));
  }
}

/// @nodoc

class _$BlockElement_QuoteImpl extends BlockElement_Quote {
  const _$BlockElement_QuoteImpl(final List<RangedBlockElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedBlockElement> _field0;
  @override
  List<RangedBlockElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'BlockElement.quote(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_QuoteImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_QuoteImplCopyWith<_$BlockElement_QuoteImpl> get copyWith =>
      __$$BlockElement_QuoteImplCopyWithImpl<_$BlockElement_QuoteImpl>(
          this, _$identity);
}

abstract class BlockElement_Quote extends BlockElement {
  const factory BlockElement_Quote(final List<RangedBlockElement> field0) =
      _$BlockElement_QuoteImpl;
  const BlockElement_Quote._() : super._();

  List<RangedBlockElement> get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_QuoteImplCopyWith<_$BlockElement_QuoteImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_UnorderedListImplCopyWith<$Res> {
  factory _$$BlockElement_UnorderedListImplCopyWith(
          _$BlockElement_UnorderedListImpl value,
          $Res Function(_$BlockElement_UnorderedListImpl) then) =
      __$$BlockElement_UnorderedListImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<List<RangedBlockElement>> field0});
}

/// @nodoc
class __$$BlockElement_UnorderedListImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_UnorderedListImpl>
    implements _$$BlockElement_UnorderedListImplCopyWith<$Res> {
  __$$BlockElement_UnorderedListImplCopyWithImpl(
      _$BlockElement_UnorderedListImpl _value,
      $Res Function(_$BlockElement_UnorderedListImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_UnorderedListImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<List<RangedBlockElement>>,
    ));
  }
}

/// @nodoc

class _$BlockElement_UnorderedListImpl extends BlockElement_UnorderedList {
  const _$BlockElement_UnorderedListImpl(
      final List<List<RangedBlockElement>> field0)
      : _field0 = field0,
        super._();

  final List<List<RangedBlockElement>> _field0;
  @override
  List<List<RangedBlockElement>> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'BlockElement.unorderedList(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_UnorderedListImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_UnorderedListImplCopyWith<_$BlockElement_UnorderedListImpl>
      get copyWith => __$$BlockElement_UnorderedListImplCopyWithImpl<
          _$BlockElement_UnorderedListImpl>(this, _$identity);
}

abstract class BlockElement_UnorderedList extends BlockElement {
  const factory BlockElement_UnorderedList(
          final List<List<RangedBlockElement>> field0) =
      _$BlockElement_UnorderedListImpl;
  const BlockElement_UnorderedList._() : super._();

  List<List<RangedBlockElement>> get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_UnorderedListImplCopyWith<_$BlockElement_UnorderedListImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_OrderedListImplCopyWith<$Res> {
  factory _$$BlockElement_OrderedListImplCopyWith(
          _$BlockElement_OrderedListImpl value,
          $Res Function(_$BlockElement_OrderedListImpl) then) =
      __$$BlockElement_OrderedListImplCopyWithImpl<$Res>;
  @useResult
  $Res call({BigInt field0, List<List<RangedBlockElement>> field1});
}

/// @nodoc
class __$$BlockElement_OrderedListImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_OrderedListImpl>
    implements _$$BlockElement_OrderedListImplCopyWith<$Res> {
  __$$BlockElement_OrderedListImplCopyWithImpl(
      _$BlockElement_OrderedListImpl _value,
      $Res Function(_$BlockElement_OrderedListImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
    Object? field1 = null,
  }) {
    return _then(_$BlockElement_OrderedListImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as BigInt,
      null == field1
          ? _value._field1
          : field1 // ignore: cast_nullable_to_non_nullable
              as List<List<RangedBlockElement>>,
    ));
  }
}

/// @nodoc

class _$BlockElement_OrderedListImpl extends BlockElement_OrderedList {
  const _$BlockElement_OrderedListImpl(
      this.field0, final List<List<RangedBlockElement>> field1)
      : _field1 = field1,
        super._();

  @override
  final BigInt field0;
  final List<List<RangedBlockElement>> _field1;
  @override
  List<List<RangedBlockElement>> get field1 {
    if (_field1 is EqualUnmodifiableListView) return _field1;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field1);
  }

  @override
  String toString() {
    return 'BlockElement.orderedList(field0: $field0, field1: $field1)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_OrderedListImpl &&
            (identical(other.field0, field0) || other.field0 == field0) &&
            const DeepCollectionEquality().equals(other._field1, _field1));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, field0, const DeepCollectionEquality().hash(_field1));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_OrderedListImplCopyWith<_$BlockElement_OrderedListImpl>
      get copyWith => __$$BlockElement_OrderedListImplCopyWithImpl<
          _$BlockElement_OrderedListImpl>(this, _$identity);
}

abstract class BlockElement_OrderedList extends BlockElement {
  const factory BlockElement_OrderedList(
          final BigInt field0, final List<List<RangedBlockElement>> field1) =
      _$BlockElement_OrderedListImpl;
  const BlockElement_OrderedList._() : super._();

  BigInt get field0;
  List<List<RangedBlockElement>> get field1;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_OrderedListImplCopyWith<_$BlockElement_OrderedListImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_TableImplCopyWith<$Res> {
  factory _$$BlockElement_TableImplCopyWith(_$BlockElement_TableImpl value,
          $Res Function(_$BlockElement_TableImpl) then) =
      __$$BlockElement_TableImplCopyWithImpl<$Res>;
  @useResult
  $Res call(
      {List<List<RangedBlockElement>> head,
      List<List<List<RangedBlockElement>>> rows});
}

/// @nodoc
class __$$BlockElement_TableImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_TableImpl>
    implements _$$BlockElement_TableImplCopyWith<$Res> {
  __$$BlockElement_TableImplCopyWithImpl(_$BlockElement_TableImpl _value,
      $Res Function(_$BlockElement_TableImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? head = null,
    Object? rows = null,
  }) {
    return _then(_$BlockElement_TableImpl(
      head: null == head
          ? _value._head
          : head // ignore: cast_nullable_to_non_nullable
              as List<List<RangedBlockElement>>,
      rows: null == rows
          ? _value._rows
          : rows // ignore: cast_nullable_to_non_nullable
              as List<List<List<RangedBlockElement>>>,
    ));
  }
}

/// @nodoc

class _$BlockElement_TableImpl extends BlockElement_Table {
  const _$BlockElement_TableImpl(
      {required final List<List<RangedBlockElement>> head,
      required final List<List<List<RangedBlockElement>>> rows})
      : _head = head,
        _rows = rows,
        super._();

  final List<List<RangedBlockElement>> _head;
  @override
  List<List<RangedBlockElement>> get head {
    if (_head is EqualUnmodifiableListView) return _head;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_head);
  }

  final List<List<List<RangedBlockElement>>> _rows;
  @override
  List<List<List<RangedBlockElement>>> get rows {
    if (_rows is EqualUnmodifiableListView) return _rows;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_rows);
  }

  @override
  String toString() {
    return 'BlockElement.table(head: $head, rows: $rows)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_TableImpl &&
            const DeepCollectionEquality().equals(other._head, _head) &&
            const DeepCollectionEquality().equals(other._rows, _rows));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType,
      const DeepCollectionEquality().hash(_head),
      const DeepCollectionEquality().hash(_rows));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_TableImplCopyWith<_$BlockElement_TableImpl> get copyWith =>
      __$$BlockElement_TableImplCopyWithImpl<_$BlockElement_TableImpl>(
          this, _$identity);
}

abstract class BlockElement_Table extends BlockElement {
  const factory BlockElement_Table(
          {required final List<List<RangedBlockElement>> head,
          required final List<List<List<RangedBlockElement>>> rows}) =
      _$BlockElement_TableImpl;
  const BlockElement_Table._() : super._();

  List<List<RangedBlockElement>> get head;
  List<List<List<RangedBlockElement>>> get rows;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_TableImplCopyWith<_$BlockElement_TableImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_HorizontalRuleImplCopyWith<$Res> {
  factory _$$BlockElement_HorizontalRuleImplCopyWith(
          _$BlockElement_HorizontalRuleImpl value,
          $Res Function(_$BlockElement_HorizontalRuleImpl) then) =
      __$$BlockElement_HorizontalRuleImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$BlockElement_HorizontalRuleImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_HorizontalRuleImpl>
    implements _$$BlockElement_HorizontalRuleImplCopyWith<$Res> {
  __$$BlockElement_HorizontalRuleImplCopyWithImpl(
      _$BlockElement_HorizontalRuleImpl _value,
      $Res Function(_$BlockElement_HorizontalRuleImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$BlockElement_HorizontalRuleImpl extends BlockElement_HorizontalRule {
  const _$BlockElement_HorizontalRuleImpl() : super._();

  @override
  String toString() {
    return 'BlockElement.horizontalRule()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_HorizontalRuleImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;
}

abstract class BlockElement_HorizontalRule extends BlockElement {
  const factory BlockElement_HorizontalRule() =
      _$BlockElement_HorizontalRuleImpl;
  const BlockElement_HorizontalRule._() : super._();
}

/// @nodoc
abstract class _$$BlockElement_CodeBlockImplCopyWith<$Res> {
  factory _$$BlockElement_CodeBlockImplCopyWith(
          _$BlockElement_CodeBlockImpl value,
          $Res Function(_$BlockElement_CodeBlockImpl) then) =
      __$$BlockElement_CodeBlockImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedCodeBlock> field0});
}

/// @nodoc
class __$$BlockElement_CodeBlockImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_CodeBlockImpl>
    implements _$$BlockElement_CodeBlockImplCopyWith<$Res> {
  __$$BlockElement_CodeBlockImplCopyWithImpl(
      _$BlockElement_CodeBlockImpl _value,
      $Res Function(_$BlockElement_CodeBlockImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_CodeBlockImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedCodeBlock>,
    ));
  }
}

/// @nodoc

class _$BlockElement_CodeBlockImpl extends BlockElement_CodeBlock {
  const _$BlockElement_CodeBlockImpl(final List<RangedCodeBlock> field0)
      : _field0 = field0,
        super._();

  final List<RangedCodeBlock> _field0;
  @override
  List<RangedCodeBlock> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'BlockElement.codeBlock(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_CodeBlockImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_CodeBlockImplCopyWith<_$BlockElement_CodeBlockImpl>
      get copyWith => __$$BlockElement_CodeBlockImplCopyWithImpl<
          _$BlockElement_CodeBlockImpl>(this, _$identity);
}

abstract class BlockElement_CodeBlock extends BlockElement {
  const factory BlockElement_CodeBlock(final List<RangedCodeBlock> field0) =
      _$BlockElement_CodeBlockImpl;
  const BlockElement_CodeBlock._() : super._();

  List<RangedCodeBlock> get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_CodeBlockImplCopyWith<_$BlockElement_CodeBlockImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BlockElement_ErrorImplCopyWith<$Res> {
  factory _$$BlockElement_ErrorImplCopyWith(_$BlockElement_ErrorImpl value,
          $Res Function(_$BlockElement_ErrorImpl) then) =
      __$$BlockElement_ErrorImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BlockElement_ErrorImplCopyWithImpl<$Res>
    extends _$BlockElementCopyWithImpl<$Res, _$BlockElement_ErrorImpl>
    implements _$$BlockElement_ErrorImplCopyWith<$Res> {
  __$$BlockElement_ErrorImplCopyWithImpl(_$BlockElement_ErrorImpl _value,
      $Res Function(_$BlockElement_ErrorImpl) _then)
      : super(_value, _then);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$BlockElement_ErrorImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$BlockElement_ErrorImpl extends BlockElement_Error {
  const _$BlockElement_ErrorImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BlockElement.error(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BlockElement_ErrorImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BlockElement_ErrorImplCopyWith<_$BlockElement_ErrorImpl> get copyWith =>
      __$$BlockElement_ErrorImplCopyWithImpl<_$BlockElement_ErrorImpl>(
          this, _$identity);
}

abstract class BlockElement_Error extends BlockElement {
  const factory BlockElement_Error(final String field0) =
      _$BlockElement_ErrorImpl;
  const BlockElement_Error._() : super._();

  String get field0;

  /// Create a copy of BlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BlockElement_ErrorImplCopyWith<_$BlockElement_ErrorImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$InlineElement {}

/// @nodoc
abstract class $InlineElementCopyWith<$Res> {
  factory $InlineElementCopyWith(
          InlineElement value, $Res Function(InlineElement) then) =
      _$InlineElementCopyWithImpl<$Res, InlineElement>;
}

/// @nodoc
class _$InlineElementCopyWithImpl<$Res, $Val extends InlineElement>
    implements $InlineElementCopyWith<$Res> {
  _$InlineElementCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$InlineElement_TextImplCopyWith<$Res> {
  factory _$$InlineElement_TextImplCopyWith(_$InlineElement_TextImpl value,
          $Res Function(_$InlineElement_TextImpl) then) =
      __$$InlineElement_TextImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$InlineElement_TextImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_TextImpl>
    implements _$$InlineElement_TextImplCopyWith<$Res> {
  __$$InlineElement_TextImplCopyWithImpl(_$InlineElement_TextImpl _value,
      $Res Function(_$InlineElement_TextImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_TextImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$InlineElement_TextImpl extends InlineElement_Text {
  const _$InlineElement_TextImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'InlineElement.text(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_TextImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_TextImplCopyWith<_$InlineElement_TextImpl> get copyWith =>
      __$$InlineElement_TextImplCopyWithImpl<_$InlineElement_TextImpl>(
          this, _$identity);
}

abstract class InlineElement_Text extends InlineElement {
  const factory InlineElement_Text(final String field0) =
      _$InlineElement_TextImpl;
  const InlineElement_Text._() : super._();

  String get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_TextImplCopyWith<_$InlineElement_TextImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_CodeImplCopyWith<$Res> {
  factory _$$InlineElement_CodeImplCopyWith(_$InlineElement_CodeImpl value,
          $Res Function(_$InlineElement_CodeImpl) then) =
      __$$InlineElement_CodeImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$InlineElement_CodeImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_CodeImpl>
    implements _$$InlineElement_CodeImplCopyWith<$Res> {
  __$$InlineElement_CodeImplCopyWithImpl(_$InlineElement_CodeImpl _value,
      $Res Function(_$InlineElement_CodeImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_CodeImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$InlineElement_CodeImpl extends InlineElement_Code {
  const _$InlineElement_CodeImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'InlineElement.code(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_CodeImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_CodeImplCopyWith<_$InlineElement_CodeImpl> get copyWith =>
      __$$InlineElement_CodeImplCopyWithImpl<_$InlineElement_CodeImpl>(
          this, _$identity);
}

abstract class InlineElement_Code extends InlineElement {
  const factory InlineElement_Code(final String field0) =
      _$InlineElement_CodeImpl;
  const InlineElement_Code._() : super._();

  String get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_CodeImplCopyWith<_$InlineElement_CodeImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_LinkImplCopyWith<$Res> {
  factory _$$InlineElement_LinkImplCopyWith(_$InlineElement_LinkImpl value,
          $Res Function(_$InlineElement_LinkImpl) then) =
      __$$InlineElement_LinkImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String destUrl, List<RangedInlineElement> children});
}

/// @nodoc
class __$$InlineElement_LinkImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_LinkImpl>
    implements _$$InlineElement_LinkImplCopyWith<$Res> {
  __$$InlineElement_LinkImplCopyWithImpl(_$InlineElement_LinkImpl _value,
      $Res Function(_$InlineElement_LinkImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? destUrl = null,
    Object? children = null,
  }) {
    return _then(_$InlineElement_LinkImpl(
      destUrl: null == destUrl
          ? _value.destUrl
          : destUrl // ignore: cast_nullable_to_non_nullable
              as String,
      children: null == children
          ? _value._children
          : children // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$InlineElement_LinkImpl extends InlineElement_Link {
  const _$InlineElement_LinkImpl(
      {required this.destUrl,
      required final List<RangedInlineElement> children})
      : _children = children,
        super._();

  @override
  final String destUrl;
  final List<RangedInlineElement> _children;
  @override
  List<RangedInlineElement> get children {
    if (_children is EqualUnmodifiableListView) return _children;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_children);
  }

  @override
  String toString() {
    return 'InlineElement.link(destUrl: $destUrl, children: $children)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_LinkImpl &&
            (identical(other.destUrl, destUrl) || other.destUrl == destUrl) &&
            const DeepCollectionEquality().equals(other._children, _children));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, destUrl, const DeepCollectionEquality().hash(_children));

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_LinkImplCopyWith<_$InlineElement_LinkImpl> get copyWith =>
      __$$InlineElement_LinkImplCopyWithImpl<_$InlineElement_LinkImpl>(
          this, _$identity);
}

abstract class InlineElement_Link extends InlineElement {
  const factory InlineElement_Link(
          {required final String destUrl,
          required final List<RangedInlineElement> children}) =
      _$InlineElement_LinkImpl;
  const InlineElement_Link._() : super._();

  String get destUrl;
  List<RangedInlineElement> get children;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_LinkImplCopyWith<_$InlineElement_LinkImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_BoldImplCopyWith<$Res> {
  factory _$$InlineElement_BoldImplCopyWith(_$InlineElement_BoldImpl value,
          $Res Function(_$InlineElement_BoldImpl) then) =
      __$$InlineElement_BoldImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$InlineElement_BoldImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_BoldImpl>
    implements _$$InlineElement_BoldImplCopyWith<$Res> {
  __$$InlineElement_BoldImplCopyWithImpl(_$InlineElement_BoldImpl _value,
      $Res Function(_$InlineElement_BoldImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_BoldImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$InlineElement_BoldImpl extends InlineElement_Bold {
  const _$InlineElement_BoldImpl(final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'InlineElement.bold(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_BoldImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_BoldImplCopyWith<_$InlineElement_BoldImpl> get copyWith =>
      __$$InlineElement_BoldImplCopyWithImpl<_$InlineElement_BoldImpl>(
          this, _$identity);
}

abstract class InlineElement_Bold extends InlineElement {
  const factory InlineElement_Bold(final List<RangedInlineElement> field0) =
      _$InlineElement_BoldImpl;
  const InlineElement_Bold._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_BoldImplCopyWith<_$InlineElement_BoldImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_ItalicImplCopyWith<$Res> {
  factory _$$InlineElement_ItalicImplCopyWith(_$InlineElement_ItalicImpl value,
          $Res Function(_$InlineElement_ItalicImpl) then) =
      __$$InlineElement_ItalicImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$InlineElement_ItalicImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_ItalicImpl>
    implements _$$InlineElement_ItalicImplCopyWith<$Res> {
  __$$InlineElement_ItalicImplCopyWithImpl(_$InlineElement_ItalicImpl _value,
      $Res Function(_$InlineElement_ItalicImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_ItalicImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$InlineElement_ItalicImpl extends InlineElement_Italic {
  const _$InlineElement_ItalicImpl(final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'InlineElement.italic(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_ItalicImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_ItalicImplCopyWith<_$InlineElement_ItalicImpl>
      get copyWith =>
          __$$InlineElement_ItalicImplCopyWithImpl<_$InlineElement_ItalicImpl>(
              this, _$identity);
}

abstract class InlineElement_Italic extends InlineElement {
  const factory InlineElement_Italic(final List<RangedInlineElement> field0) =
      _$InlineElement_ItalicImpl;
  const InlineElement_Italic._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_ItalicImplCopyWith<_$InlineElement_ItalicImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_StrikethroughImplCopyWith<$Res> {
  factory _$$InlineElement_StrikethroughImplCopyWith(
          _$InlineElement_StrikethroughImpl value,
          $Res Function(_$InlineElement_StrikethroughImpl) then) =
      __$$InlineElement_StrikethroughImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$InlineElement_StrikethroughImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_StrikethroughImpl>
    implements _$$InlineElement_StrikethroughImplCopyWith<$Res> {
  __$$InlineElement_StrikethroughImplCopyWithImpl(
      _$InlineElement_StrikethroughImpl _value,
      $Res Function(_$InlineElement_StrikethroughImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_StrikethroughImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$InlineElement_StrikethroughImpl extends InlineElement_Strikethrough {
  const _$InlineElement_StrikethroughImpl(
      final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'InlineElement.strikethrough(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_StrikethroughImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_StrikethroughImplCopyWith<_$InlineElement_StrikethroughImpl>
      get copyWith => __$$InlineElement_StrikethroughImplCopyWithImpl<
          _$InlineElement_StrikethroughImpl>(this, _$identity);
}

abstract class InlineElement_Strikethrough extends InlineElement {
  const factory InlineElement_Strikethrough(
          final List<RangedInlineElement> field0) =
      _$InlineElement_StrikethroughImpl;
  const InlineElement_Strikethrough._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_StrikethroughImplCopyWith<_$InlineElement_StrikethroughImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_SpoilerImplCopyWith<$Res> {
  factory _$$InlineElement_SpoilerImplCopyWith(
          _$InlineElement_SpoilerImpl value,
          $Res Function(_$InlineElement_SpoilerImpl) then) =
      __$$InlineElement_SpoilerImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<RangedInlineElement> field0});
}

/// @nodoc
class __$$InlineElement_SpoilerImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_SpoilerImpl>
    implements _$$InlineElement_SpoilerImplCopyWith<$Res> {
  __$$InlineElement_SpoilerImplCopyWithImpl(_$InlineElement_SpoilerImpl _value,
      $Res Function(_$InlineElement_SpoilerImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_SpoilerImpl(
      null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<RangedInlineElement>,
    ));
  }
}

/// @nodoc

class _$InlineElement_SpoilerImpl extends InlineElement_Spoiler {
  const _$InlineElement_SpoilerImpl(final List<RangedInlineElement> field0)
      : _field0 = field0,
        super._();

  final List<RangedInlineElement> _field0;
  @override
  List<RangedInlineElement> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'InlineElement.spoiler(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_SpoilerImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_SpoilerImplCopyWith<_$InlineElement_SpoilerImpl>
      get copyWith => __$$InlineElement_SpoilerImplCopyWithImpl<
          _$InlineElement_SpoilerImpl>(this, _$identity);
}

abstract class InlineElement_Spoiler extends InlineElement {
  const factory InlineElement_Spoiler(final List<RangedInlineElement> field0) =
      _$InlineElement_SpoilerImpl;
  const InlineElement_Spoiler._() : super._();

  List<RangedInlineElement> get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_SpoilerImplCopyWith<_$InlineElement_SpoilerImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_ImageImplCopyWith<$Res> {
  factory _$$InlineElement_ImageImplCopyWith(_$InlineElement_ImageImpl value,
          $Res Function(_$InlineElement_ImageImpl) then) =
      __$$InlineElement_ImageImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$InlineElement_ImageImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res, _$InlineElement_ImageImpl>
    implements _$$InlineElement_ImageImplCopyWith<$Res> {
  __$$InlineElement_ImageImplCopyWithImpl(_$InlineElement_ImageImpl _value,
      $Res Function(_$InlineElement_ImageImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_ImageImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$InlineElement_ImageImpl extends InlineElement_Image {
  const _$InlineElement_ImageImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'InlineElement.image(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_ImageImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_ImageImplCopyWith<_$InlineElement_ImageImpl> get copyWith =>
      __$$InlineElement_ImageImplCopyWithImpl<_$InlineElement_ImageImpl>(
          this, _$identity);
}

abstract class InlineElement_Image extends InlineElement {
  const factory InlineElement_Image(final String field0) =
      _$InlineElement_ImageImpl;
  const InlineElement_Image._() : super._();

  String get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_ImageImplCopyWith<_$InlineElement_ImageImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$InlineElement_TaskListMarkerImplCopyWith<$Res> {
  factory _$$InlineElement_TaskListMarkerImplCopyWith(
          _$InlineElement_TaskListMarkerImpl value,
          $Res Function(_$InlineElement_TaskListMarkerImpl) then) =
      __$$InlineElement_TaskListMarkerImplCopyWithImpl<$Res>;
  @useResult
  $Res call({bool field0});
}

/// @nodoc
class __$$InlineElement_TaskListMarkerImplCopyWithImpl<$Res>
    extends _$InlineElementCopyWithImpl<$Res,
        _$InlineElement_TaskListMarkerImpl>
    implements _$$InlineElement_TaskListMarkerImplCopyWith<$Res> {
  __$$InlineElement_TaskListMarkerImplCopyWithImpl(
      _$InlineElement_TaskListMarkerImpl _value,
      $Res Function(_$InlineElement_TaskListMarkerImpl) _then)
      : super(_value, _then);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$InlineElement_TaskListMarkerImpl(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as bool,
    ));
  }
}

/// @nodoc

class _$InlineElement_TaskListMarkerImpl extends InlineElement_TaskListMarker {
  const _$InlineElement_TaskListMarkerImpl(this.field0) : super._();

  @override
  final bool field0;

  @override
  String toString() {
    return 'InlineElement.taskListMarker(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$InlineElement_TaskListMarkerImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$InlineElement_TaskListMarkerImplCopyWith<
          _$InlineElement_TaskListMarkerImpl>
      get copyWith => __$$InlineElement_TaskListMarkerImplCopyWithImpl<
          _$InlineElement_TaskListMarkerImpl>(this, _$identity);
}

abstract class InlineElement_TaskListMarker extends InlineElement {
  const factory InlineElement_TaskListMarker(final bool field0) =
      _$InlineElement_TaskListMarkerImpl;
  const InlineElement_TaskListMarker._() : super._();

  bool get field0;

  /// Create a copy of InlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$InlineElement_TaskListMarkerImplCopyWith<
          _$InlineElement_TaskListMarkerImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$MessageContent {
  List<RangedBlockElement> get content => throw _privateConstructorUsedError;

  /// Create a copy of MessageContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $MessageContentCopyWith<MessageContent> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $MessageContentCopyWith<$Res> {
  factory $MessageContentCopyWith(
          MessageContent value, $Res Function(MessageContent) then) =
      _$MessageContentCopyWithImpl<$Res, MessageContent>;
  @useResult
  $Res call({List<RangedBlockElement> content});
}

/// @nodoc
class _$MessageContentCopyWithImpl<$Res, $Val extends MessageContent>
    implements $MessageContentCopyWith<$Res> {
  _$MessageContentCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of MessageContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? content = null,
  }) {
    return _then(_value.copyWith(
      content: null == content
          ? _value.content
          : content // ignore: cast_nullable_to_non_nullable
              as List<RangedBlockElement>,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$MessageContentImplCopyWith<$Res>
    implements $MessageContentCopyWith<$Res> {
  factory _$$MessageContentImplCopyWith(_$MessageContentImpl value,
          $Res Function(_$MessageContentImpl) then) =
      __$$MessageContentImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({List<RangedBlockElement> content});
}

/// @nodoc
class __$$MessageContentImplCopyWithImpl<$Res>
    extends _$MessageContentCopyWithImpl<$Res, _$MessageContentImpl>
    implements _$$MessageContentImplCopyWith<$Res> {
  __$$MessageContentImplCopyWithImpl(
      _$MessageContentImpl _value, $Res Function(_$MessageContentImpl) _then)
      : super(_value, _then);

  /// Create a copy of MessageContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? content = null,
  }) {
    return _then(_$MessageContentImpl(
      content: null == content
          ? _value._content
          : content // ignore: cast_nullable_to_non_nullable
              as List<RangedBlockElement>,
    ));
  }
}

/// @nodoc

class _$MessageContentImpl extends _MessageContent {
  const _$MessageContentImpl({required final List<RangedBlockElement> content})
      : _content = content,
        super._();

  final List<RangedBlockElement> _content;
  @override
  List<RangedBlockElement> get content {
    if (_content is EqualUnmodifiableListView) return _content;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_content);
  }

  @override
  String toString() {
    return 'MessageContent(content: $content)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$MessageContentImpl &&
            const DeepCollectionEquality().equals(other._content, _content));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_content));

  /// Create a copy of MessageContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$MessageContentImplCopyWith<_$MessageContentImpl> get copyWith =>
      __$$MessageContentImplCopyWithImpl<_$MessageContentImpl>(
          this, _$identity);
}

abstract class _MessageContent extends MessageContent {
  const factory _MessageContent(
      {required final List<RangedBlockElement> content}) = _$MessageContentImpl;
  const _MessageContent._() : super._();

  @override
  List<RangedBlockElement> get content;

  /// Create a copy of MessageContent
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$MessageContentImplCopyWith<_$MessageContentImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$RangedBlockElement {
  int get start => throw _privateConstructorUsedError;
  int get end => throw _privateConstructorUsedError;
  BlockElement get element => throw _privateConstructorUsedError;

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $RangedBlockElementCopyWith<RangedBlockElement> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $RangedBlockElementCopyWith<$Res> {
  factory $RangedBlockElementCopyWith(
          RangedBlockElement value, $Res Function(RangedBlockElement) then) =
      _$RangedBlockElementCopyWithImpl<$Res, RangedBlockElement>;
  @useResult
  $Res call({int start, int end, BlockElement element});

  $BlockElementCopyWith<$Res> get element;
}

/// @nodoc
class _$RangedBlockElementCopyWithImpl<$Res, $Val extends RangedBlockElement>
    implements $RangedBlockElementCopyWith<$Res> {
  _$RangedBlockElementCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? element = null,
  }) {
    return _then(_value.copyWith(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      element: null == element
          ? _value.element
          : element // ignore: cast_nullable_to_non_nullable
              as BlockElement,
    ) as $Val);
  }

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $BlockElementCopyWith<$Res> get element {
    return $BlockElementCopyWith<$Res>(_value.element, (value) {
      return _then(_value.copyWith(element: value) as $Val);
    });
  }
}

/// @nodoc
abstract class _$$RangedBlockElementImplCopyWith<$Res>
    implements $RangedBlockElementCopyWith<$Res> {
  factory _$$RangedBlockElementImplCopyWith(_$RangedBlockElementImpl value,
          $Res Function(_$RangedBlockElementImpl) then) =
      __$$RangedBlockElementImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({int start, int end, BlockElement element});

  @override
  $BlockElementCopyWith<$Res> get element;
}

/// @nodoc
class __$$RangedBlockElementImplCopyWithImpl<$Res>
    extends _$RangedBlockElementCopyWithImpl<$Res, _$RangedBlockElementImpl>
    implements _$$RangedBlockElementImplCopyWith<$Res> {
  __$$RangedBlockElementImplCopyWithImpl(_$RangedBlockElementImpl _value,
      $Res Function(_$RangedBlockElementImpl) _then)
      : super(_value, _then);

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? element = null,
  }) {
    return _then(_$RangedBlockElementImpl(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      element: null == element
          ? _value.element
          : element // ignore: cast_nullable_to_non_nullable
              as BlockElement,
    ));
  }
}

/// @nodoc

class _$RangedBlockElementImpl implements _RangedBlockElement {
  const _$RangedBlockElementImpl(
      {required this.start, required this.end, required this.element});

  @override
  final int start;
  @override
  final int end;
  @override
  final BlockElement element;

  @override
  String toString() {
    return 'RangedBlockElement(start: $start, end: $end, element: $element)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RangedBlockElementImpl &&
            (identical(other.start, start) || other.start == start) &&
            (identical(other.end, end) || other.end == end) &&
            (identical(other.element, element) || other.element == element));
  }

  @override
  int get hashCode => Object.hash(runtimeType, start, end, element);

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RangedBlockElementImplCopyWith<_$RangedBlockElementImpl> get copyWith =>
      __$$RangedBlockElementImplCopyWithImpl<_$RangedBlockElementImpl>(
          this, _$identity);
}

abstract class _RangedBlockElement implements RangedBlockElement {
  const factory _RangedBlockElement(
      {required final int start,
      required final int end,
      required final BlockElement element}) = _$RangedBlockElementImpl;

  @override
  int get start;
  @override
  int get end;
  @override
  BlockElement get element;

  /// Create a copy of RangedBlockElement
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RangedBlockElementImplCopyWith<_$RangedBlockElementImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$RangedCodeBlock {
  int get start => throw _privateConstructorUsedError;
  int get end => throw _privateConstructorUsedError;
  String get value => throw _privateConstructorUsedError;

  /// Create a copy of RangedCodeBlock
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $RangedCodeBlockCopyWith<RangedCodeBlock> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $RangedCodeBlockCopyWith<$Res> {
  factory $RangedCodeBlockCopyWith(
          RangedCodeBlock value, $Res Function(RangedCodeBlock) then) =
      _$RangedCodeBlockCopyWithImpl<$Res, RangedCodeBlock>;
  @useResult
  $Res call({int start, int end, String value});
}

/// @nodoc
class _$RangedCodeBlockCopyWithImpl<$Res, $Val extends RangedCodeBlock>
    implements $RangedCodeBlockCopyWith<$Res> {
  _$RangedCodeBlockCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of RangedCodeBlock
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? value = null,
  }) {
    return _then(_value.copyWith(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      value: null == value
          ? _value.value
          : value // ignore: cast_nullable_to_non_nullable
              as String,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$RangedCodeBlockImplCopyWith<$Res>
    implements $RangedCodeBlockCopyWith<$Res> {
  factory _$$RangedCodeBlockImplCopyWith(_$RangedCodeBlockImpl value,
          $Res Function(_$RangedCodeBlockImpl) then) =
      __$$RangedCodeBlockImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({int start, int end, String value});
}

/// @nodoc
class __$$RangedCodeBlockImplCopyWithImpl<$Res>
    extends _$RangedCodeBlockCopyWithImpl<$Res, _$RangedCodeBlockImpl>
    implements _$$RangedCodeBlockImplCopyWith<$Res> {
  __$$RangedCodeBlockImplCopyWithImpl(
      _$RangedCodeBlockImpl _value, $Res Function(_$RangedCodeBlockImpl) _then)
      : super(_value, _then);

  /// Create a copy of RangedCodeBlock
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? value = null,
  }) {
    return _then(_$RangedCodeBlockImpl(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      value: null == value
          ? _value.value
          : value // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$RangedCodeBlockImpl implements _RangedCodeBlock {
  const _$RangedCodeBlockImpl(
      {required this.start, required this.end, required this.value});

  @override
  final int start;
  @override
  final int end;
  @override
  final String value;

  @override
  String toString() {
    return 'RangedCodeBlock(start: $start, end: $end, value: $value)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RangedCodeBlockImpl &&
            (identical(other.start, start) || other.start == start) &&
            (identical(other.end, end) || other.end == end) &&
            (identical(other.value, value) || other.value == value));
  }

  @override
  int get hashCode => Object.hash(runtimeType, start, end, value);

  /// Create a copy of RangedCodeBlock
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RangedCodeBlockImplCopyWith<_$RangedCodeBlockImpl> get copyWith =>
      __$$RangedCodeBlockImplCopyWithImpl<_$RangedCodeBlockImpl>(
          this, _$identity);
}

abstract class _RangedCodeBlock implements RangedCodeBlock {
  const factory _RangedCodeBlock(
      {required final int start,
      required final int end,
      required final String value}) = _$RangedCodeBlockImpl;

  @override
  int get start;
  @override
  int get end;
  @override
  String get value;

  /// Create a copy of RangedCodeBlock
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RangedCodeBlockImplCopyWith<_$RangedCodeBlockImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$RangedInlineElement {
  int get start => throw _privateConstructorUsedError;
  int get end => throw _privateConstructorUsedError;
  InlineElement get element => throw _privateConstructorUsedError;

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $RangedInlineElementCopyWith<RangedInlineElement> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $RangedInlineElementCopyWith<$Res> {
  factory $RangedInlineElementCopyWith(
          RangedInlineElement value, $Res Function(RangedInlineElement) then) =
      _$RangedInlineElementCopyWithImpl<$Res, RangedInlineElement>;
  @useResult
  $Res call({int start, int end, InlineElement element});

  $InlineElementCopyWith<$Res> get element;
}

/// @nodoc
class _$RangedInlineElementCopyWithImpl<$Res, $Val extends RangedInlineElement>
    implements $RangedInlineElementCopyWith<$Res> {
  _$RangedInlineElementCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? element = null,
  }) {
    return _then(_value.copyWith(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      element: null == element
          ? _value.element
          : element // ignore: cast_nullable_to_non_nullable
              as InlineElement,
    ) as $Val);
  }

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $InlineElementCopyWith<$Res> get element {
    return $InlineElementCopyWith<$Res>(_value.element, (value) {
      return _then(_value.copyWith(element: value) as $Val);
    });
  }
}

/// @nodoc
abstract class _$$RangedInlineElementImplCopyWith<$Res>
    implements $RangedInlineElementCopyWith<$Res> {
  factory _$$RangedInlineElementImplCopyWith(_$RangedInlineElementImpl value,
          $Res Function(_$RangedInlineElementImpl) then) =
      __$$RangedInlineElementImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({int start, int end, InlineElement element});

  @override
  $InlineElementCopyWith<$Res> get element;
}

/// @nodoc
class __$$RangedInlineElementImplCopyWithImpl<$Res>
    extends _$RangedInlineElementCopyWithImpl<$Res, _$RangedInlineElementImpl>
    implements _$$RangedInlineElementImplCopyWith<$Res> {
  __$$RangedInlineElementImplCopyWithImpl(_$RangedInlineElementImpl _value,
      $Res Function(_$RangedInlineElementImpl) _then)
      : super(_value, _then);

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? start = null,
    Object? end = null,
    Object? element = null,
  }) {
    return _then(_$RangedInlineElementImpl(
      start: null == start
          ? _value.start
          : start // ignore: cast_nullable_to_non_nullable
              as int,
      end: null == end
          ? _value.end
          : end // ignore: cast_nullable_to_non_nullable
              as int,
      element: null == element
          ? _value.element
          : element // ignore: cast_nullable_to_non_nullable
              as InlineElement,
    ));
  }
}

/// @nodoc

class _$RangedInlineElementImpl implements _RangedInlineElement {
  const _$RangedInlineElementImpl(
      {required this.start, required this.end, required this.element});

  @override
  final int start;
  @override
  final int end;
  @override
  final InlineElement element;

  @override
  String toString() {
    return 'RangedInlineElement(start: $start, end: $end, element: $element)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RangedInlineElementImpl &&
            (identical(other.start, start) || other.start == start) &&
            (identical(other.end, end) || other.end == end) &&
            (identical(other.element, element) || other.element == element));
  }

  @override
  int get hashCode => Object.hash(runtimeType, start, end, element);

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RangedInlineElementImplCopyWith<_$RangedInlineElementImpl> get copyWith =>
      __$$RangedInlineElementImplCopyWithImpl<_$RangedInlineElementImpl>(
          this, _$identity);
}

abstract class _RangedInlineElement implements RangedInlineElement {
  const factory _RangedInlineElement(
      {required final int start,
      required final int end,
      required final InlineElement element}) = _$RangedInlineElementImpl;

  @override
  int get start;
  @override
  int get end;
  @override
  InlineElement get element;

  /// Create a copy of RangedInlineElement
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RangedInlineElementImplCopyWith<_$RangedInlineElementImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
