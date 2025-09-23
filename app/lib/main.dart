// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:air/app.dart';
import 'package:air/core/frb_generated.dart';
import 'package:air/core/core.dart';
import 'package:air/ui/colors/palette.dart';
import 'package:air/ui/colors/themes.dart';
import 'package:air/util/logging.dart';
import 'package:path/path.dart' as p;

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await RustLib.init();

  final cacheDir = await getApplicationCacheDirectory();
  final logFile = p.join(cacheDir.path, 'app.log');

  final logWriter = initRustLogging(logFile: logFile);
  initLogging(logWriter);

  runApp(const App());
}

void showErrorBanner(BuildContext context, String errorDescription) {
  ScaffoldMessenger.of(context).showMaterialBanner(
    MaterialBanner(
      backgroundColor: AppColors.red,
      leading: const Icon(Icons.error),
      padding: const EdgeInsets.all(20),
      content: Text(errorDescription),
      actions: [
        TextButton(
          child: Text(
            'OK',
            style: TextStyle(
              color: CustomColorScheme.of(context).function.white,
            ),
          ),
          onPressed: () {
            ScaffoldMessenger.of(context).hideCurrentMaterialBanner();
          },
        ),
      ],
    ),
  );
}
