// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.9.0.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:freezed_annotation/freezed_annotation.dart' hide protected;
part 'markdown.freezed.dart';

// These functions are ignored because they are not marked as `pub`: `parse_block_element`, `parse_inline_elements`, `parse_list_items`, `parse_table_cells`, `parse_table_content`, `try_parse_markdown`
// These types are ignored because they are neither used by any `pub` functions nor (for structs and enums) marked `#[frb(unignore)]`: `Error`, `RangedEvent`
// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `assert_receiver_is_total_eq`, `clone`, `clone`, `clone`, `clone`, `clone`, `clone`, `clone`, `eq`, `eq`, `eq`, `eq`, `eq`, `eq`, `eq`, `eq`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `fmt`, `hash`, `hash`, `hash`, `hash`, `hash`, `hash`

@freezed
sealed class BlockElement with _$BlockElement {
  const BlockElement._();

  const factory BlockElement.paragraph(List<RangedInlineElement> field0) =
      BlockElement_Paragraph;
  const factory BlockElement.heading(List<RangedInlineElement> field0) =
      BlockElement_Heading;
  const factory BlockElement.quote(List<RangedBlockElement> field0) =
      BlockElement_Quote;
  const factory BlockElement.unorderedList(
    List<List<RangedBlockElement>> field0,
  ) = BlockElement_UnorderedList;
  const factory BlockElement.orderedList(
    BigInt field0,
    List<List<RangedBlockElement>> field1,
  ) = BlockElement_OrderedList;
  const factory BlockElement.table({
    required List<List<RangedBlockElement>> head,
    required List<List<List<RangedBlockElement>>> rows,
  }) = BlockElement_Table;
  const factory BlockElement.horizontalRule() = BlockElement_HorizontalRule;

  /// If code blocks are indented, each line is a separate String
  const factory BlockElement.codeBlock(List<RangedCodeBlock> field0) =
      BlockElement_CodeBlock;
  const factory BlockElement.error(String field0) = BlockElement_Error;
}

@freezed
sealed class InlineElement with _$InlineElement {
  const InlineElement._();

  const factory InlineElement.text(String field0) = InlineElement_Text;
  const factory InlineElement.code(String field0) = InlineElement_Code;
  const factory InlineElement.link({
    required String destUrl,
    required List<RangedInlineElement> children,
  }) = InlineElement_Link;
  const factory InlineElement.bold(List<RangedInlineElement> field0) =
      InlineElement_Bold;
  const factory InlineElement.italic(List<RangedInlineElement> field0) =
      InlineElement_Italic;
  const factory InlineElement.strikethrough(List<RangedInlineElement> field0) =
      InlineElement_Strikethrough;
  const factory InlineElement.spoiler(List<RangedInlineElement> field0) =
      InlineElement_Spoiler;
  const factory InlineElement.image(String field0) = InlineElement_Image;
  const factory InlineElement.taskListMarker(bool field0) =
      InlineElement_TaskListMarker;
}

@freezed
class MessageContent with _$MessageContent {
  const MessageContent._();
  const factory MessageContent({required List<RangedBlockElement> content}) =
      _MessageContent;
  static Future<MessageContent> error({required String message}) => RustLib
      .instance
      .api
      .crateApiMarkdownMessageContentError(message: message);

  static Future<MessageContent> parseMarkdown({required String string}) =>
      RustLib.instance.api.crateApiMarkdownMessageContentParseMarkdown(
        string: string,
      );

  static MessageContent parseMarkdownRaw({required List<int> string}) => RustLib
      .instance
      .api
      .crateApiMarkdownMessageContentParseMarkdownRaw(string: string);
}

@freezed
class RangedBlockElement with _$RangedBlockElement {
  const factory RangedBlockElement({
    required int start,
    required int end,
    required BlockElement element,
  }) = _RangedBlockElement;
}

@freezed
class RangedCodeBlock with _$RangedCodeBlock {
  const factory RangedCodeBlock({
    required int start,
    required int end,
    required String value,
  }) = _RangedCodeBlock;
}

@freezed
class RangedInlineElement with _$RangedInlineElement {
  const factory RangedInlineElement({
    required int start,
    required int end,
    required InlineElement element,
  }) = _RangedInlineElement;
}
