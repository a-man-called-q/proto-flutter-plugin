import 'package:example_mobile_app/main.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('renders the shared UI package', (tester) async {
    await tester.pumpWidget(const ExampleApp());

    expect(find.text('Hello, Flutter monorepo!'), findsOneWidget);
  });
}
