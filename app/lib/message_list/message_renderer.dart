// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/theme/theme.dart';

enum HostWidget { textField, richText }

TextSpan _styledTextSpan(
    String keyword, TextStyle? style, HostWidget hostWidget) {
  return TextSpan(
    text: hostWidget == HostWidget.textField
        ? "\u200B" * (keyword.length - 1)
        : keyword,
    style: hostWidget == HostWidget.richText
        ? const TextStyle(
            fontSize: 0,
            color: Colors.transparent,
          )
        : style,
    children: [
      WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        child: Transform.translate(
          offset: Offset(0, hostWidget == HostWidget.richText ? -6 : 0),
          child: Container(
            height: 22,
            decoration: BoxDecoration(
              //color: colorDMBLight,
              color: colorGreyLight,
              borderRadius: BorderRadius.circular(4),
            ),
            padding: const EdgeInsets.fromLTRB(5, 0, 5, 0),
            margin: const EdgeInsets.symmetric(horizontal: 1, vertical: 5),
            child: SelectionContainer.disabled(
              child: Text(
                keyword,
                style: style
                    ?.copyWith(
                      //color: colorDMB,
                      fontSize: 12,
                    )
                    .merge(VariableFontWeight.semiBold),
              ),
            ),
          ),
        ),
      ),
    ],
  );
}

TextSpan buildTextSpanFromText(List<String> keywords, String text,
    TextStyle? style, HostWidget hostWidget) {
  // We match all the keywords in the text
  final matches = keywords
      .map((keyword) => RegExp(keyword, caseSensitive: false).allMatches(text))
      .expand((element) => element)
      .toList();

  if (matches.isEmpty) {
    return TextSpan(
      style: style,
      text: text,
    );
  }

  // We sort the matches in increasing order
  matches.sort((a, b) => a.start.compareTo(b.start));

  // Then we calculate the parts of the string that are not covered by the
  // matches
  final List<TextSpan> children = [];
  int lastMatchEnd = 0;

  for (final match in matches) {
    if (match.start > lastMatchEnd) {
      children.add(TextSpan(
        text: text.substring(lastMatchEnd, match.start),
        style: style,
      ));
    }
    children.add(_styledTextSpan(match.group(0)!, style, hostWidget));
    lastMatchEnd = match.end;
  }
  if (lastMatchEnd < text.length) {
    children.add(TextSpan(
      text: text.substring(lastMatchEnd),
      style: style,
    ));
  }

  /*  final splits = text.split(keyword);

  if (splits.length <= 1) {
    return TextSpan(style: style, text: text);
  }

  final first = splits.first;

  List<TextSpan> spans = splits.skip(1).expand((e) {
    final styledSpan = _styledTextSpan(keyword, style, hostWidget);
    return e.length > 0
        ? [styledSpan, TextSpan(style: style, text: e)]
        : [styledSpan];
  }).toList();

  spans.insert(0, TextSpan(style: style, text: first));
 */
  return TextSpan(
    style: style,
    children: children,
  );
}
