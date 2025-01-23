// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';
import 'package:prototype/core/core.dart';

/// Initializes Dart and Rust logging
///
/// Also configures the format of the logs.
void initLogging() {
  // Init Dart logging
  Logger.root.level = kDebugMode ? Level.FINE : Level.INFO;
  Logger.root.onRecord.listen((record) {
    print(
        '[F] ${record.time} ${record.level.name} ${record.loggerName}: ${record.message}');
  });

  // Rust Logging
  createLogStream().listen((event) {
    print(
        '[R] ${event.time.toLocal()} ${event.level.asString} ${event.target}: ${event.msg}');
  });
}

extension on LogEntryLevel {
  String get asString => switch (this) {
        LogEntryLevel.trace => 'TRACE',
        LogEntryLevel.debug => 'DEBUG',
        LogEntryLevel.info => ' INFO',
        LogEntryLevel.warn => ' WARN',
        LogEntryLevel.error => 'ERROR'
      };
}
