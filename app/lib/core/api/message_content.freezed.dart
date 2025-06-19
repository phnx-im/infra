// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'message_content.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiMimiContent {

 Uint8List? get replaces; Uint8List get topicId; Uint8List? get inReplyTo; String get plainBody; MessageContent get content; List<UiAttachment> get attachments;
/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMimiContentCopyWith<UiMimiContent> get copyWith => _$UiMimiContentCopyWithImpl<UiMimiContent>(this as UiMimiContent, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMimiContent&&const DeepCollectionEquality().equals(other.replaces, replaces)&&const DeepCollectionEquality().equals(other.topicId, topicId)&&const DeepCollectionEquality().equals(other.inReplyTo, inReplyTo)&&(identical(other.plainBody, plainBody) || other.plainBody == plainBody)&&(identical(other.content, content) || other.content == content)&&const DeepCollectionEquality().equals(other.attachments, attachments));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(replaces),const DeepCollectionEquality().hash(topicId),const DeepCollectionEquality().hash(inReplyTo),plainBody,content,const DeepCollectionEquality().hash(attachments));

@override
String toString() {
  return 'UiMimiContent(replaces: $replaces, topicId: $topicId, inReplyTo: $inReplyTo, plainBody: $plainBody, content: $content, attachments: $attachments)';
}


}

/// @nodoc
abstract mixin class $UiMimiContentCopyWith<$Res>  {
  factory $UiMimiContentCopyWith(UiMimiContent value, $Res Function(UiMimiContent) _then) = _$UiMimiContentCopyWithImpl;
@useResult
$Res call({
 Uint8List? replaces, Uint8List topicId, Uint8List? inReplyTo, String plainBody, MessageContent content, List<UiAttachment> attachments
});


$MessageContentCopyWith<$Res> get content;

}
/// @nodoc
class _$UiMimiContentCopyWithImpl<$Res>
    implements $UiMimiContentCopyWith<$Res> {
  _$UiMimiContentCopyWithImpl(this._self, this._then);

  final UiMimiContent _self;
  final $Res Function(UiMimiContent) _then;

/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? replaces = freezed,Object? topicId = null,Object? inReplyTo = freezed,Object? plainBody = null,Object? content = null,Object? attachments = null,}) {
  return _then(_self.copyWith(
replaces: freezed == replaces ? _self.replaces : replaces // ignore: cast_nullable_to_non_nullable
as Uint8List?,topicId: null == topicId ? _self.topicId : topicId // ignore: cast_nullable_to_non_nullable
as Uint8List,inReplyTo: freezed == inReplyTo ? _self.inReplyTo : inReplyTo // ignore: cast_nullable_to_non_nullable
as Uint8List?,plainBody: null == plainBody ? _self.plainBody : plainBody // ignore: cast_nullable_to_non_nullable
as String,content: null == content ? _self.content : content // ignore: cast_nullable_to_non_nullable
as MessageContent,attachments: null == attachments ? _self.attachments : attachments // ignore: cast_nullable_to_non_nullable
as List<UiAttachment>,
  ));
}
/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$MessageContentCopyWith<$Res> get content {
  
  return $MessageContentCopyWith<$Res>(_self.content, (value) {
    return _then(_self.copyWith(content: value));
  });
}
}


/// @nodoc


class _UiMimiContent implements UiMimiContent {
  const _UiMimiContent({this.replaces, required this.topicId, this.inReplyTo, required this.plainBody, required this.content, required final  List<UiAttachment> attachments}): _attachments = attachments;
  

@override final  Uint8List? replaces;
@override final  Uint8List topicId;
@override final  Uint8List? inReplyTo;
@override final  String plainBody;
@override final  MessageContent content;
 final  List<UiAttachment> _attachments;
@override List<UiAttachment> get attachments {
  if (_attachments is EqualUnmodifiableListView) return _attachments;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_attachments);
}


/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$UiMimiContentCopyWith<_UiMimiContent> get copyWith => __$UiMimiContentCopyWithImpl<_UiMimiContent>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _UiMimiContent&&const DeepCollectionEquality().equals(other.replaces, replaces)&&const DeepCollectionEquality().equals(other.topicId, topicId)&&const DeepCollectionEquality().equals(other.inReplyTo, inReplyTo)&&(identical(other.plainBody, plainBody) || other.plainBody == plainBody)&&(identical(other.content, content) || other.content == content)&&const DeepCollectionEquality().equals(other._attachments, _attachments));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(replaces),const DeepCollectionEquality().hash(topicId),const DeepCollectionEquality().hash(inReplyTo),plainBody,content,const DeepCollectionEquality().hash(_attachments));

@override
String toString() {
  return 'UiMimiContent(replaces: $replaces, topicId: $topicId, inReplyTo: $inReplyTo, plainBody: $plainBody, content: $content, attachments: $attachments)';
}


}

/// @nodoc
abstract mixin class _$UiMimiContentCopyWith<$Res> implements $UiMimiContentCopyWith<$Res> {
  factory _$UiMimiContentCopyWith(_UiMimiContent value, $Res Function(_UiMimiContent) _then) = __$UiMimiContentCopyWithImpl;
@override @useResult
$Res call({
 Uint8List? replaces, Uint8List topicId, Uint8List? inReplyTo, String plainBody, MessageContent content, List<UiAttachment> attachments
});


@override $MessageContentCopyWith<$Res> get content;

}
/// @nodoc
class __$UiMimiContentCopyWithImpl<$Res>
    implements _$UiMimiContentCopyWith<$Res> {
  __$UiMimiContentCopyWithImpl(this._self, this._then);

  final _UiMimiContent _self;
  final $Res Function(_UiMimiContent) _then;

/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? replaces = freezed,Object? topicId = null,Object? inReplyTo = freezed,Object? plainBody = null,Object? content = null,Object? attachments = null,}) {
  return _then(_UiMimiContent(
replaces: freezed == replaces ? _self.replaces : replaces // ignore: cast_nullable_to_non_nullable
as Uint8List?,topicId: null == topicId ? _self.topicId : topicId // ignore: cast_nullable_to_non_nullable
as Uint8List,inReplyTo: freezed == inReplyTo ? _self.inReplyTo : inReplyTo // ignore: cast_nullable_to_non_nullable
as Uint8List?,plainBody: null == plainBody ? _self.plainBody : plainBody // ignore: cast_nullable_to_non_nullable
as String,content: null == content ? _self.content : content // ignore: cast_nullable_to_non_nullable
as MessageContent,attachments: null == attachments ? _self._attachments : attachments // ignore: cast_nullable_to_non_nullable
as List<UiAttachment>,
  ));
}

/// Create a copy of UiMimiContent
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$MessageContentCopyWith<$Res> get content {
  
  return $MessageContentCopyWith<$Res>(_self.content, (value) {
    return _then(_self.copyWith(content: value));
  });
}
}

// dart format on
