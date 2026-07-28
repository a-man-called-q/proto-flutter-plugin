import 'package:example_ui/ui.dart';
import 'package:flutter/material.dart';

void main() {
  runApp(const ExampleApp());
}

class ExampleApp extends StatelessWidget {
  const ExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      title: 'Flutter monorepo example',
      home: Scaffold(
        body: Center(child: GreetingCard(name: 'Flutter monorepo')),
      ),
    );
  }
}
