// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:prototype/app.dart';
import 'package:prototype/core/frb_generated.dart';
import 'package:prototype/core/core.dart';
import 'package:prototype/util/logging.dart';
import 'package:path/path.dart' as p;

class Foobar {
  const Foobar({
    required this.value,
    required this.name,
    required this.id,
    required this.date,
  });

  final int value;
}

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await RustLib.init();

  final cacheDir = await getApplicationCacheDirectory();
  final logFile = p.join(cacheDir.path, 'app.log');

  final logWriter = initRustLogging(logFile: logFile);
  initLogging(logWriter);

  runApp(const App());
}

void showErrorBanner(
  ScaffoldMessengerState messengerState,
  String errorDescription,
) {
  messengerState.showMaterialBanner(
    MaterialBanner(
      backgroundColor: Colors.red,
      leading: const Icon(Icons.error),
      padding: const EdgeInsets.all(20),
      content: Text(errorDescription),
      actions: [
        TextButton(
          child: const Text('OK', style: TextStyle(color: Colors.white)),
          onPressed: () {
            messengerState.hideCurrentMaterialBanner();
          },
        ),
      ],
    ),
  );
}
