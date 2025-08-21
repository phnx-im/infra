// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:air/core/api/markdown.dart';
import 'package:air/ui/colors/palette.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/ui/typography/font_size.dart';

Widget buildBlockElement(
  BuildContext context,
  BlockElement block,
  bool isSender,
) {
  return switch (block) {
    BlockElement_Paragraph(:final field0) => Text.rich(
      TextSpan(
        children:
            field0
                .map((child) => buildInlineElement(context, child, isSender))
                .toList(),
        style: TextStyle(
          color:
              isSender
                  ? CustomColorScheme.of(context).message.selfText
                  : CustomColorScheme.of(context).message.otherText,
          fontSize: BodyFontSize.base.size,
        ),
      ),
    ),
    BlockElement_Heading(:final field0) => Text.rich(
      TextSpan(
        children:
            field0
                .map((child) => buildInlineElement(context, child, isSender))
                .toList(),
        style: TextStyle(
          fontSize: BodyFontSize.large1.size,
          fontWeight: FontWeight.bold,
          color:
              isSender
                  ? CustomColorScheme.of(context).backgroundBase.primary
                  : CustomColorScheme.of(context).text.primary,
        ),
      ),
    ),
    BlockElement_Quote(:final field0) => Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: const Border(left: BorderSide(color: AppColors.blue, width: 4)),
        color: AppColors.blue[700],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children:
            field0
                .map(
                  (inner) =>
                      buildBlockElement(context, inner.element, isSender),
                )
                .toList(),
      ),
    ),
    BlockElement_UnorderedList(:final field0) => Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children:
          field0
              .map(
                (items) => Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text.rich(TextSpan(text: " \u2022  ")),
                    Flexible(
                      child: Column(
                        spacing: 4.0,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children:
                            items
                                .map(
                                  (item) => buildBlockElement(
                                    context,
                                    item.element,
                                    isSender,
                                  ),
                                )
                                .toList(),
                      ),
                    ),
                  ],
                ),
              )
              .toList(),
    ),
    BlockElement_OrderedList(:final field0, :final field1) => Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children:
          field1.indexed
              .map(
                (items) => Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text.rich(
                      TextSpan(
                        text: " ${field0 + BigInt.from(items.$1)}.  ",
                        style: TextStyle(
                          color:
                              isSender
                                  ? AppColors.blue[100]
                                  : AppColors.blue[900],
                          fontSize: BodyFontSize.base.size,
                        ),
                      ),
                    ),
                    Flexible(
                      child: Column(
                        spacing: 4.0,
                        mainAxisAlignment: MainAxisAlignment.start,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children:
                            items.$2
                                .map(
                                  (item) => buildBlockElement(
                                    context,
                                    item.element,
                                    isSender,
                                  ),
                                )
                                .toList(),
                      ),
                    ),
                  ],
                ),
              )
              .toList(),
    ),
    BlockElement_Table(:final head, :final rows) => Table(
      border: TableBorder.all(),
      defaultColumnWidth: const FlexColumnWidth(),
      children: [
        TableRow(
          children:
              head
                  .map(
                    (itemBlocks) => Column(
                      children:
                          itemBlocks
                              .map(
                                (item) => buildBlockElement(
                                  context,
                                  item.element,
                                  isSender,
                                ),
                              )
                              .toList(),
                    ),
                  )
                  .toList(),
        ),
        ...rows.map(
          (row) => TableRow(
            children:
                row
                    .map(
                      (itemBlocks) => Column(
                        children:
                            itemBlocks
                                .map(
                                  (item) => buildBlockElement(
                                    context,
                                    item.element,
                                    isSender,
                                  ),
                                )
                                .toList(),
                      ),
                    )
                    .toList(),
          ),
        ),
      ],
    ),
    BlockElement_HorizontalRule() => Divider(
      color:
          isSender
              ? CustomColorScheme.of(context).message.selfText
              : CustomColorScheme.of(context).message.otherText,
    ),
    BlockElement_CodeBlock(:final field0) => Text.rich(
      TextSpan(
        text: field0.map((e) => e.value).join('\n'),
        style: const TextStyle(fontFamily: 'SourceCodeProEmbedded'),
      ),
    ),
    BlockElement_Error(:final field0) => Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(
            color: CustomColorScheme.of(context).separator.primary,
            width: 4,
          ),
        ),
        color: CustomColorScheme.of(context).function.warning,
      ),
      child: Text.rich(TextSpan(text: field0)),
    ),
  };
}

InlineSpan buildInlineElement(
  BuildContext context,
  RangedInlineElement inline,
  bool isSender,
) {
  return switch (inline.element) {
    InlineElement_Text(:final field0) => TextSpan(text: field0),
    InlineElement_Code(:final field0) => TextSpan(
      text: field0,
      style: const TextStyle(fontFamily: 'SourceCodeProEmbedded'),
    ),
    InlineElement_Link(:final children) => TextSpan(
      children:
          children
              .map((child) => buildInlineElement(context, child, isSender))
              .toList(),
      style: TextStyle(
        color:
            isSender
                ? const Color(0xFF69d1ff)
                : CustomColorScheme.of(context).function.link,
        decorationColor:
            isSender
                ? const Color(0xFF69d1ff)
                : CustomColorScheme.of(context).function.link,
        decoration: TextDecoration.underline,
      ),
    ),
    InlineElement_Bold(:final field0) => TextSpan(
      children:
          field0
              .map((child) => buildInlineElement(context, child, isSender))
              .toList(),
      style: const TextStyle(fontWeight: FontWeight.bold),
    ),
    InlineElement_Italic(:final field0) => TextSpan(
      children:
          field0
              .map((child) => buildInlineElement(context, child, isSender))
              .toList(),
      style: const TextStyle(fontStyle: FontStyle.italic),
    ),
    InlineElement_Strikethrough(:final field0) => TextSpan(
      children:
          field0
              .map((child) => buildInlineElement(context, child, isSender))
              .toList(),
      style: const TextStyle(decoration: TextDecoration.lineThrough),
    ),
    InlineElement_Spoiler(:final field0) => TextSpan(
      children:
          field0
              .map((child) => buildInlineElement(context, child, isSender))
              .toList(),
      style: TextStyle(
        decoration: TextDecoration.combine([
          TextDecoration.overline,
          TextDecoration.lineThrough,
          TextDecoration.underline,
        ]),
      ),
    ),
    InlineElement_Image() => const WidgetSpan(child: Icon(Icons.image)),
    InlineElement_TaskListMarker(:final field0) => WidgetSpan(
      alignment: PlaceholderAlignment.middle,
      child: Padding(
        padding: const EdgeInsets.only(right: 8),
        child: SizedBox(
          width: 20,
          height: 20,
          child: Checkbox(value: field0, onChanged: null),
        ),
      ),
    ),
  };
}

// The style used for formatting characters like * or >
TextStyle highlightStyle(BuildContext context) =>
    TextStyle(color: CustomColorScheme.of(context).function.link);

class CustomTextEditingController extends TextEditingController {
  // Keep track of where widgets are, so the cursor can treat it as one unit
  List<({int start, int end})> widgetRanges = [];
  int lastKnownRawTextLength = 0;
  int previousCursorPosition = 0;
  Uint8List raw = Uint8List(0);

  CustomTextEditingController() {
    addListener(_handleCursorMovement);
  }

  void _handleCursorMovement() {
    int cursorPosition = selection.extentOffset;

    if (cursorPosition == -1) {
      return;
    }

    if (lastKnownRawTextLength < text.length) {
      // Do nothing when writing text
      previousCursorPosition = cursorPosition;
      return;
    }

    // Convert position into UTF-8 index
    String charsUpToCursor = text.substring(0, cursorPosition);
    int cursorPositionUtf8 = utf8.encode(charsUpToCursor).length;

    if (lastKnownRawTextLength > text.length) {
      // Was part of a widget deleted? Then either:
      // - The user pressed backspace, so the cursor is now at the end of where the widget was
      // - The user pressed delete, so the cursor is still at the character just before where the widget was

      for (var range in widgetRanges) {
        if (cursorPosition >= range.start && cursorPosition < range.end) {
          int startUtf16 = utf8.decode(raw.sublist(0, range.start)).length;

          if (cursorPosition != previousCursorPosition) {
            // The cursor moved, so this was a backspace and not a delete
            var newText = text.replaceRange(startUtf16, cursorPosition, "");

            // Make sure we don't use outdated data
            widgetRanges.clear();
            lastKnownRawTextLength = newText.length;

            text = newText;

            moveCursorTo(startUtf16);
          } else {
            // The cursor did not move, this was a delete, not a backspace
            int endUtf16 = utf8.decode(raw.sublist(0, range.end)).length;
            var removedChars = lastKnownRawTextLength - text.length;
            var newText = text.replaceRange(
              cursorPosition,
              endUtf16 - removedChars,
              "",
            );

            // Make sure we don't use outdated data
            widgetRanges.clear();
            lastKnownRawTextLength = newText.length;

            text = newText;

            moveCursorTo(startUtf16);
          }

          break;
        }
      }

      previousCursorPosition = cursorPosition;
      return;
    }

    for (var range in widgetRanges) {
      // If the cursor is inside a widget range, push it to the end
      if (cursorPositionUtf8 > range.start && cursorPositionUtf8 < range.end) {
        if (cursorPosition < previousCursorPosition) {
          int startUtf16 = utf8.decode(raw.sublist(0, range.start)).length;
          moveCursorTo(startUtf16);
        } else {
          int endUtf16 = utf8.decode(raw.sublist(0, range.end)).length;
          moveCursorTo(endUtf16);
        }

        break;
      }
    }
    previousCursorPosition = cursorPosition;
  }

  void moveCursorTo(int newPosition) {
    Future.delayed(Duration.zero, () {
      previousCursorPosition = newPosition;
      if (selection.baseOffset == selection.extentOffset) {
        // Move cursor, don't start selection
        selection = TextSelection(
          extentOffset: newPosition,
          baseOffset: newPosition,
          affinity: selection.affinity,
          isDirectional: selection.isDirectional,
        );
      } else {
        // Keep baseOffset the same to continue selection
        selection = TextSelection(
          extentOffset: newPosition,
          baseOffset: selection.baseOffset,
          affinity: selection.affinity,
          isDirectional: selection.isDirectional,
        );
      }
    });
  }

  @override
  TextSpan buildTextSpan({
    required context,
    TextStyle? style,
    required bool withComposing,
  }) {
    // Regenerating this data
    widgetRanges.clear();
    lastKnownRawTextLength = text.length;

    // Flutter uses UTF-16, but Rust uses UTF-8
    raw = utf8.encode(text);

    MessageContent parsed = const MessageContent(elements: []);

    if (text.isNotEmpty) {
      parsed = MessageContent.parseMarkdownRaw(string: raw);
    }

    return TextSpan(
      style: style,
      children: buildWrappedBlock(context, 0, raw.length, parsed.elements),
    );
  }

  InlineSpan buildFormattedTextSpanBlock(
    BuildContext context,
    RangedBlockElement block,
  ) {
    return switch (block.element) {
      BlockElement_Paragraph(:final field0) => TextSpan(
        children: buildWrappedInline(context, block.start, block.end, field0),
      ),
      BlockElement_Heading(:final field0) => TextSpan(
        children: buildWrappedInline(context, block.start, block.end, field0),
        style: const TextStyle(fontSize: 20),
      ),
      BlockElement_Quote(:final field0) => TextSpan(
        children: buildWrappedBlock(context, block.start, block.end, field0),
        style: TextStyle(color: AppColors.neutral[600]),
      ),
      BlockElement_UnorderedList(:final field0) => TextSpan(
        children: buildWrappedBlock(
          context,
          block.start,
          block.end,
          field0.expand((list) => list).toList(),
        ),
      ),
      BlockElement_OrderedList(:final field1) => TextSpan(
        children: buildWrappedBlock(
          context,
          block.start,
          block.end,
          field1.expand((list) => list).toList(),
        ),
      ),
      BlockElement_Table() => TextSpan(
        text: utf8.decode(raw.sublist(block.start, block.end)),
        style: highlightStyle(context),
      ),
      BlockElement_HorizontalRule() => TextSpan(
        text: utf8.decode(raw.sublist(block.start, block.end)),
        style: highlightStyle(context),
      ),
      BlockElement_CodeBlock(:final field0) => TextSpan(
        children: buildWrappedInline(
          context,
          block.start,
          block.end,
          field0
              .map(
                (item) => RangedInlineElement(
                  start: item.start,
                  end: item.end,
                  element: InlineElement.code(item.value),
                ),
              )
              .toList(),
        ),
        style: const TextStyle(fontFamily: 'SourceCodeProEmbedded'),
      ),
      BlockElement_Error() => TextSpan(
        text: utf8.decode(raw.sublist(block.start, block.end)),
        style: TextStyle(
          color: CustomColorScheme.of(context).function.danger,
          decorationColor: CustomColorScheme.of(context).function.danger,
          decoration: TextDecoration.underline,
          decorationStyle: TextDecorationStyle.wavy,
        ),
      ),
    };
  }

  InlineSpan buildFormattedTextSpanInline(
    BuildContext context,
    RangedInlineElement inline,
  ) {
    return switch (inline.element) {
      // TODO: Handle this case.
      InlineElement_Text() => TextSpan(
        text: utf8.decode(raw.sublist(inline.start, inline.end)),
      ),
      InlineElement_Code() => TextSpan(
        text: utf8.decode(raw.sublist(inline.start, inline.end)),
        style: const TextStyle(fontFamily: 'SourceCodeProEmbedded'),
      ),
      InlineElement_Link() => TextSpan(
        text: utf8.decode(raw.sublist(inline.start, inline.end)),
        style: TextStyle(
          color: CustomColorScheme.of(context).function.link,
          decorationColor: CustomColorScheme.of(context).function.link,
          decoration: TextDecoration.underline,
        ),
      ),
      InlineElement_Bold(:final field0) => TextSpan(
        children: buildWrappedInline(context, inline.start, inline.end, field0),
        style: const TextStyle(fontWeight: FontWeight.bold),
      ),
      InlineElement_Italic(:final field0) => TextSpan(
        children: buildWrappedInline(context, inline.start, inline.end, field0),
        style: const TextStyle(fontStyle: FontStyle.italic),
      ),
      InlineElement_Strikethrough(:final field0) => TextSpan(
        children: buildWrappedInline(context, inline.start, inline.end, field0),
        style: const TextStyle(decoration: TextDecoration.lineThrough),
      ),
      InlineElement_Spoiler(:final field0) => TextSpan(
        children: buildWrappedInline(context, inline.start, inline.end, field0),
        style: TextStyle(
          decoration: TextDecoration.combine([
            TextDecoration.overline,
            TextDecoration.lineThrough,
            TextDecoration.underline,
          ]),
        ),
      ),
      InlineElement_Image() => buildCorrectWidget(
        const SizedBox(height: 14, width: 32, child: Icon(Icons.image)),
        inline.start,
        inline.end,
      ),
      InlineElement_TaskListMarker() => TextSpan(
        text: utf8.decode(raw.sublist(inline.start, inline.end)),
        style: highlightStyle(context),
      ),
    };
  }

  InlineSpan buildCorrectWidget(Widget widget, int rangeStart, int rangeEnd) {
    widgetRanges.add((start: rangeStart, end: rangeEnd));

    return TextSpan(
      children: [
        WidgetSpan(child: widget),
        TextSpan(text: "\u200d" * (rangeEnd - rangeStart - 1)),
      ],
    );
  }

  List<InlineSpan> buildWrappedInline(
    BuildContext context,
    int rangeStart,
    int rangeEnd,
    List<RangedInlineElement> value,
  ) {
    List<InlineSpan> children = [];

    var lastInner = (start: 0, end: rangeStart);

    for (var inner in value) {
      if (inner.start < rangeStart) {
        // This element is outside of the surrounding block. Ignore.
        // This can happen for this markdown: "- [ ] > test"
        continue;
      }
      // Gap between previous and this inline
      if (lastInner.end < inner.start) {
        children.add(
          TextSpan(
            text: utf8.decode(raw.sublist(lastInner.end, inner.start)),
            style: highlightStyle(context),
          ),
        );
      }

      children.add(buildFormattedTextSpanInline(context, inner));
      lastInner = (start: inner.start, end: inner.end);
    }

    // Gap after last inline
    if (lastInner.end < rangeEnd) {
      children.add(
        TextSpan(
          text: utf8.decode(raw.sublist(lastInner.end, rangeEnd)),
          style: highlightStyle(context),
        ),
      );
    }

    return children;
  }

  List<InlineSpan> buildWrappedBlock(
    BuildContext context,
    int rangeStart,
    int rangeEnd,
    List<RangedBlockElement> value,
  ) {
    List<InlineSpan> children = [];

    var lastInner = (start: 0, end: rangeStart);

    for (var inner in value) {
      // Gap between previous and this block
      if (lastInner.end < inner.start) {
        children.add(
          TextSpan(
            text: utf8.decode(raw.sublist(lastInner.end, inner.start)),
            style: highlightStyle(context),
          ),
        );
      }

      children.add(buildFormattedTextSpanBlock(context, inner));

      lastInner = (start: inner.start, end: inner.end);
    }

    // Gap after last block
    if (lastInner.end < rangeEnd) {
      children.add(
        TextSpan(
          text: utf8.decode(raw.sublist(lastInner.end, rangeEnd)),
          style: highlightStyle(context),
        ),
      );
    }

    return children;
  }
}
