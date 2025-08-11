import 'dart:io';
import 'package:flutter/material.dart';

String getSystemMonospaceFontFamily() {
  if (Platform.isWindows) return 'Consolas';
  if (Platform.isMacOS || Platform.isIOS) return 'Menlo';
  if (Platform.isLinux) return 'monospace';
  if (Platform.isAndroid) return 'monospace';
  return 'monospace';
}

List<String>? getSystemMonospaceFontFallback() {
  return null;
}

extension SystemMonospaceTextStyle on TextStyle {
  TextStyle withSystemMonospace() => copyWith(
    fontFamily: getSystemMonospaceFontFamily(),
    fontFamilyFallback: getSystemMonospaceFontFallback(),
  );
}
