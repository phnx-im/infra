import 'dart:math';

import 'app_localizations.dart';

extension AppLocalizationsExtension on AppLocalizations {
  String bytesToHumanReadable(int bytes) {
    const List<String> byteUnits = [
      'B',
      'KB',
      'MB',
      'GB',
      'TB',
      'PB',
      'EB',
      'ZB',
      'YB',
    ];

    if (bytes == 0) {
      return '0 B';
    }

    int i = (log(bytes) / log(1000)).floor();
    i = min(i, byteUnits.length - 1);
    double value = bytes / pow(1000, i);

    return this.attachmentSize(value, byteUnits[i]);
  }
}
