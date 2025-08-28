// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:prototype/theme/theme.dart';

void main() {
  group('Fonts', () {
    Widget buildSubject() => MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: lightTheme,
      home: const Scaffold(body: Center(child: Text('Hello World'))),
    );

    testWidgets('loading', (tester) async {
      await tester.pumpWidget(buildSubject());

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/fonts.png'),
      );
    });
  });
}
