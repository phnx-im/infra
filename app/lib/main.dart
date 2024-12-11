// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:prototype/app.dart';
import 'package:prototype/core/api/mobile_logging.dart';
import 'package:prototype/core/frb_generated.dart';

void main() async {
  _initRust();
  _initLogging();

  runApp(const App());
}

void _initLogging() {
  Logger.root.level = kDebugMode ? Level.FINE : Level.INFO;
  Logger.root.onRecord.listen((record) {
    print('${record.level.name}: ${record.time}: ${record.message}');
  });
}

Future<void> _initRust() async {
  // FRB
  await RustLib.init();
  // Logging
  createLogStream().listen((event) {
    print('Rust: ${event.level} ${event.tag} ${event.msg} ${event.timeMillis}');
  });
}

void showErrorBanner(BuildContext context, String errorDescription) {
  ScaffoldMessenger.of(context).showMaterialBanner(
    MaterialBanner(
      backgroundColor: Colors.red,
      leading: const Icon(Icons.error),
      padding: const EdgeInsets.all(20),
      content: Text(errorDescription),
      actions: [
        TextButton(
          child: const Text(
            'OK',
            style: TextStyle(color: Colors.white),
          ),
          onPressed: () {
            ScaffoldMessenger.of(context).hideCurrentMaterialBanner();
          },
        ),
      ],
    ),
  );
}
