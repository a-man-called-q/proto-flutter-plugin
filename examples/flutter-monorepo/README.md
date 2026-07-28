# Flutter monorepo example

This executable example combines:

- Proto and this plugin for one pinned Flutter SDK;
- a Dart Pub workspace for shared dependency resolution;
- the native Flutter Moon toolchain for dependency installation, project
  discovery, task inference, and the project graph.

The workspace contains one Flutter app, one Flutter UI package, and one pure
Dart package:

```text
mobile_app -> ui -> core
           -> core
```

## Run from this repository

Build both local plugins first:

```sh
cd ../..
moon run :build
cd examples/flutter-monorepo
```

Install Flutter and run the inferred checks:

```sh
proto install flutter
proto run --exe dart flutter -- pub workspace list
moon query projects
moon run :analyze :test
```

Moon runs `flutter pub get` once at the Pub workspace root through its dependency
installation lifecycle. The package project files only define their layer;
aliases, dependencies, and tasks are inferred from `pubspec.yaml`.

Run the web app locally:

```sh
moon run mobile_app:run -- -d chrome
```

## Copy into another repository

The checked-in `.prototools` uses the local debug WASM build so CI tests the
current source. Replace its plugin locator when copying this example:

```toml
proto = "0.59.0"
moon = "2.4.5"
flutter = "3.44.8"

[plugins.tools]
flutter = "github://KonstantinKai/proto-flutter-plugin"
```

Replace the local toolchain locator in `.moon/toolchains.yml`:

```yaml
flutter:
  plugin: "github://KonstantinKai/proto-flutter-plugin/flutter_toolchain@v0.4.0"
  versionFromPrototools: true
  inferTasks: true
```

Proto owns Flutter installation and version selection. Pub owns package
resolution. Moon owns dependency installation, project discovery, dependency
ordering, caching, and task execution.
