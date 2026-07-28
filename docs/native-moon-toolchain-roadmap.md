# Native Flutter toolchain for Moon

Released in **v0.4.0**.

The v0.3.2 monorepo example originally integrated Flutter through regular Moon
tasks. v0.4.0 adds an opt-in native toolchain while preserving that task-based
integration for existing users.

## Architecture

- Convert the repository to a Cargo workspace without changing the existing
  proto plugin's public interface or minimum proto version.
- Keep the proto plugin as `flutter_tool.wasm`.
- Add a separate crate and release artifact named
  `flutter_toolchain.wasm`, built with `moon_pdk`.
- Reuse the same Proto hooks inside the Moon toolchain artifact. Moon creates
  the Proto tool with ID `flutter`, so both artifacts share the SDK layout and
  resolver behavior.

## Toolchain capabilities

- Detect Dart and Flutter projects from `pubspec.yaml`.
- Derive project aliases from `name` and dependency relationships from workspace
  package dependencies.
- Expose the proto-managed `flutter` and `dart` executables.
- Register `pubspec.yaml`, the root `pubspec.lock`, and relevant Dart sources as
  task inputs.
- Provide default `pub-get`, `analyze`, and `test` tasks, with interactive
  `run` tasks excluded from CI and caching.
- Initialize through:

  ```sh
  moon toolchain add flutter <plugin-locator>
  ```

Moon `2.4.5` and `moon_pdk` `2.0.4` are the compatibility baseline. The Moon
PDK remains experimental, so its dependencies are pinned exactly and forward
compatibility is checked separately.
