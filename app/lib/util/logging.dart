// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';
import 'package:air/core/core.dart';

/// Initializes Dart and Rust logging
///
/// Also configures the format of the logs.
void initLogging(LogWriter logWriter) {
  // Dart logging
  Logger.root.level = kDebugMode ? Level.FINE : Level.INFO;
  Logger.root.onRecord.listen((record) {
    final message =
        '${record.time} [Dart] ${record.level.asString} ${record.loggerName}: ${record.message}';
    // ignore: avoid_print
    print(message);
    logWriter.writeLine(message: message);
  });

  // Rust Logging
  createLogStream().listen((event) {
    // ignore: avoid_print
    print(
      '${event.time.toLocal()} [Rust] ${event.level.asString} ${event.target}: ${event.msg}',
    );
  });
}

extension on Level {
  String get asString => switch (this) {
    Level.ALL => 'ALL',
    Level.OFF => 'OFF',
    Level.SHOUT => 'SHOUT',
    Level.SEVERE => 'SEVERE',
    Level.WARNING => ' WARN',
    Level.INFO => ' INFO',
    Level.CONFIG => 'CONFIG',
    Level.FINE => ' FINE',
    Level.FINER => 'FINER',
    Level.FINEST => 'FINEST',
    _ => 'UNKNOWN',
  };
}

extension on LogEntryLevel {
  String get asString => switch (this) {
    LogEntryLevel.trace => 'TRACE',
    LogEntryLevel.debug => 'DEBUG',
    LogEntryLevel.info => ' INFO',
    LogEntryLevel.warn => ' WARN',
    LogEntryLevel.error => 'ERROR',
  };
}
