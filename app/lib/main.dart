// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:prototype/app.dart';
import 'package:prototype/core/frb_generated.dart';
import 'package:prototype/util/logging.dart';

void main() async {
  await RustLib.init();
  initLogging();

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
          child: const Text(
            'OK',
            style: TextStyle(color: Colors.white),
          ),
          onPressed: () {
            messengerState.hideCurrentMaterialBanner();
          },
        ),
      ],
    ),
  );
}
