import 'package:example_ui/ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('renders a greeting from the core package', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: GreetingCard(name: 'Moon'))),
    );

    expect(find.text('Hello, Moon!'), findsOneWidget);
  });
}
