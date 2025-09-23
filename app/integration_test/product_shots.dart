import 'package:integration_test/integration_test.dart';

import '../test/product_shots/product_shots.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  run(outputBase: "product_shots");
}
