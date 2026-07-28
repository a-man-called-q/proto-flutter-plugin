use crate::FlutterToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk_api::*;
use schematic::SchemaBuilder;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "Flutter".into(),
        description: Some(
            "Flutter and Dart projects backed by a Proto-managed Flutter SDK.".into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::other("dart").map_err(|error| Error::msg(error.to_string()))?),
        config_file_globs: vec![
            "analysis_options.yaml".into(),
            "analysis_options.yml".into(),
            ".fvmrc".into(),
        ],
        exe_names: vec!["flutter".into(), "dart".into()],
        lock_file_names: vec!["pubspec.lock".into()],
        manifest_file_names: vec!["pubspec.yaml".into()],
        vendor_dir_name: Some(".dart_tool".into()),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some(
            "https://github.com/KonstantinKai/proto-flutter-plugin#native-moon-toolchain"
                .into(),
        ),
        docs_url: Some(
            "https://github.com/KonstantinKai/proto-flutter-plugin/tree/main/examples/flutter-monorepo"
                .into(),
        ),
        default_settings: [("inferTasks".into(), true.into())].into_iter().collect(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<FlutterToolchainConfig>(),
    }))
}
