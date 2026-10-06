# Flutter plugin

[![Release](https://github.com/a-man-called-q/proto-flutter-plugin/actions/workflows/release.yml/badge.svg)](https://github.com/a-man-called-q/proto-flutter-plugin/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A community [WASM plugin](https://moonrepo.dev/docs/proto/wasm-plugin) for [proto](https://github.com/moonrepo/proto) that manages [Flutter](https://flutter.dev/) SDK versions.

Requires [proto](https://github.com/moonrepo/proto) >= 0.60.0 (validated with 0.62.3)

## Installation

```sh
proto plugin add flutter "github://a-man-called-q/proto-flutter-plugin"
proto install flutter
```

Or add manually to `.prototools`:

```toml
[plugins.tools]
flutter = "github://a-man-called-q/proto-flutter-plugin"
```

## Usage

```sh
# Install Flutter
proto install flutter 3.29

# Use Flutter
proto run flutter -- --version

# List available versions
proto versions flutter

# Pin a version in the current directory
proto pin flutter 3.29
```

Invoking Flutter without arguments is also supported:

```sh
proto run flutter --
```

## Version Detection

The plugin automatically detects Flutter versions from:

- `.fvmrc` — reads `flutter` field ([FVM](https://fvm.app/) configuration file)
- `pubspec.yaml` / `pubspec.yml` — reads `environment.flutter` field (supports version constraints)

## Configuration

Configure in `.prototools` under `[tools.flutter]`:

```toml
[tools.flutter]
# Custom base URL for Flutter SDK archives (default: official Google storage)
base-url = "https://storage.googleapis.com/flutter_infra_release/releases"
```

## Supported Platforms

| Platform | Architecture | Notes |
|----------|-------------|-------|
| Linux | x64 | Versions with an official matching archive |
| macOS | x64 | Versions with an official matching archive |
| macOS | arm64 | Versions with an official matching archive |
| Windows | x64 | Versions with an official matching archive |

## Notes

- Version 0.4.1 requires proto >= 0.60.0 because the WASM plugin context and
  configuration APIs changed in proto 0.60. Proto 0.62.3 is covered by CI.
- Supports version aliases: `stable`, `beta`, `latest`
- Does not support channel switching via `flutter channel` — use `proto install flutter beta` instead
- Only includes compatible stable and beta channel versions with non-zero MAJOR part
- Respects platform and architecture compatibility when listing versions

Running `flutter channel` to list channels is supported. Passing a channel to
switch the installed SDK is blocked because proto-managed installations are
versioned and immutable.

## Moonrepo integration

v0.4.0 provides an opt-in native Moon toolchain. Keep Flutter pinned in
`.prototools`, then register the separate Moon WASM artifact. It requires Moon
>= `2.5.0` and is validated with Moon `2.6.0`; `moon_pdk` is pinned because its
WASM API is still experimental.

```yaml
# .moon/toolchains.yml
flutter:
  plugin: "github://a-man-called-q/proto-flutter-plugin/flutter_toolchain@flutter_toolchain-v0.4.0"
  versionFromPrototools: true
  inferTasks: true
```

Initialize the same configuration through the CLI with:

```sh
moon toolchain add flutter \
  "github://a-man-called-q/proto-flutter-plugin/flutter_toolchain@flutter_toolchain-v0.4.0"
```

The native toolchain detects Moon projects containing `pubspec.yaml`, derives
aliases and local dependencies, runs one `flutter pub get` per Pub workspace,
and infers:

- `analyze` for every Dart or Flutter package;
- `test` when the project contains Dart test files;
- non-cached, non-CI `run` for Flutter applications with `lib/main.dart`.

For a complete working setup with one Flutter app, shared Dart and Flutter
packages, a Pub workspace, a Moon dependency graph, and CI-ready tasks, see the
[Flutter monorepo example](examples/flutter-monorepo).

Pin Flutter and register the plugin in the consumer repository:

```toml
flutter = "3.44.8"

[plugins.tools]
flutter = "github://a-man-called-q/proto-flutter-plugin"
```

Existing task-based integration remains supported without enabling the native
toolchain. Define project tasks in `moon.yml`:

```yaml
language: "unknown"

tasks:
  analyze:
    command: "proto run flutter -- analyze"

  test:
    command: "proto run flutter -- test"

  run:
    command: "proto run flutter -- run"
    options:
      cache: false
      runInCI: false
      persistent: true
```

Install the pinned SDK once, then use the tasks normally:

```sh
proto install
moon run <project>:analyze
moon run <project>:test
moon run <project>:run
```

In both setups, Proto owns the Flutter SDK and version, Pub owns package
resolution, and Moon owns dependency ordering, caching, and task execution.
The native architecture and compatibility baseline are documented in the
[native toolchain design](docs/native-moon-toolchain-roadmap.md).

## Contributing

The development toolchain is pinned with proto. Install it and use Moon for the
standard workflow:

```sh
proto install
moon run :ci
```

The local `.prototools` points Flutter at the debug WASM build, so the plugin can
also be smoke-tested directly:

```sh
moon run :build
proto --log trace versions flutter --aliases
proto run flutter 3.44.8 -- --version
```

To register this local debug plugin for all projects on the machine, run the
following from the repository root. The absolute locator is intentional; update
it if the repository is moved:

```sh
proto plugin add flutter "file://$(pwd)/target/wasm32-wasip1/debug/flutter_tool.wasm" --to global
proto install flutter 3.44.8 --pin global
proto plugin info flutter
proto versions flutter --aliases
proto run flutter 3.44.8 -- --version
proto run --exe dart flutter 3.44.8 -- --version
```

`--to global` writes to `~/.proto/.prototools`, which proto loads from any
working directory. The `--to user` alternative writes to `~/.prototools`
instead, but proto only reads that config while the working directory is inside
the home directory, so projects on other volumes (for example `/Volumes/...`)
never see plugins registered there.

Live tests download upstream metadata and Flutter archives, and are intentionally
excluded from the default CI suite:

```sh
cargo test --all-targets --features live-tests
```

## Support

If you find this plugin useful, please give it a star on GitHub — it helps others discover the project!
