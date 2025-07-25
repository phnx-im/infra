// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.11.1.

// ignore_for_file: unreachable_switch_default, prefer_const_constructors
import 'package:convert/convert.dart';

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:uuid/uuid.dart';
import 'types.dart';

Future<void> deleteDatabases({required String dbPath}) =>
    RustLib.instance.api.crateApiUtilsDeleteDatabases(dbPath: dbPath);

Future<void> deleteClientDatabase({
  required String dbPath,
  required UiUserId userId,
}) => RustLib.instance.api.crateApiUtilsDeleteClientDatabase(
  dbPath: dbPath,
  userId: userId,
);
