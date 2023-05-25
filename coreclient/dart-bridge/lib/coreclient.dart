// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import 'dart:ffi';

import 'src/bridge_generated.dart';

export 'src/bridge_definitions.dart';
export 'src/bridge_generated.dart';

void main() async {
  final bridge = RustBridgeImpl(DynamicLibrary.process());
  await bridge.initLib();
}
