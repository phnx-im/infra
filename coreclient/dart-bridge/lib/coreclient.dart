import 'dart:ffi';

import 'src/bridge_generated.dart';

export 'src/bridge_definitions.dart';
export 'src/bridge_generated.dart';

void main() async {
  final bridge = RustBridgeImpl(DynamicLibrary.process());
  await bridge.initLib();
}
