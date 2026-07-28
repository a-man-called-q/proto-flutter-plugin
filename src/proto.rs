use std::collections::HashMap;

use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use serde::Deserialize;

use crate::{FlutterDist, FlutterDistVersion, FlutterPluginConfig, Fvmrc, PubspecYaml};

static NAME: &str = "Flutter";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        minimum_proto_version: Some(Version::new(0, 47, 0)),
        type_of: PluginType::CommandLine,
        default_install_strategy: InstallStrategy::DownloadPrebuilt,
        config_schema: Some(SchemaBuilder::build_root::<FlutterPluginConfig>()),
        self_upgrade_commands: vec!["upgrade".into(), "downgrade".into()],
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let env = get_host_environment()?;
    ensure_supported_host(&env)?;
    let response = fetch_dist(&env)?;

    Ok(Json(build_versions_output(&env, &response)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version_spec = input.context.version.as_ref();

    ensure_supported_host(&env)?;

    if version_spec.is_canary() {
        return Err(plugin_err!(PluginError::Message(format!(
            "{NAME} does not support canary/nightly versions. Please use `proto install flutter beta` instead"
        ))));
    }

    let base_url = get_tool_config::<FlutterPluginConfig>()?.base_url;
    let response = fetch_dist(&env)?;
    let release = select_release(&env, &response, version_spec)?;
    let download_url = build_download_url(&base_url, release);

    Ok(Json(DownloadPrebuiltOutput {
        download_url,
        checksum: Some(release.sha256.clone()),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "flutter".into(),
                ExecutableConfig::new_primary(
                    env.os
                        .for_native("flutter/bin/flutter", "flutter/bin/flutter.bat"),
                ),
            ),
            (
                "dart".into(),
                ExecutableConfig::new(env.os.for_native(
                    "flutter/bin/cache/dart-sdk/bin/dart",
                    "flutter/bin/cache/dart-sdk/bin/dart.exe",
                )),
            ),
        ]),
        globals_lookup_dirs: vec!["$FLUTTER_ROOT/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".fvmrc".into(), "pubspec.yml".into(), "pubspec.yaml".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn pre_run(Json(input): Json<RunHook>) -> FnResult<Json<RunHookResult>> {
    validate_run_args(&input.passthrough_args)?;

    Ok(Json(RunHookResult::default()))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<CompatibleParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".fvmrc" {
        let fvmrc: Fvmrc = json::from_str(&input.content)?;

        if let Some(flutter_version) = fvmrc.flutter {
            version = Some(UnresolvedVersionSpec::parse(flutter_version)?);
        }
    } else if input.file.starts_with("pubspec") {
        let pubspec: PubspecYaml = serde_yml::from_str(&input.content)?;

        if let Some(env) = pubspec.environment {
            if let Some(constraint) = env.flutter {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

// Proto 0.47 used `ToolContext` here, while newer Proto releases use an
// unresolved context. The plugin only needs the file name and contents, so a
// narrow input keeps this hook compatible with both host shapes.
#[derive(Deserialize)]
pub struct CompatibleParseVersionFileInput {
    pub content: String,
    pub file: String,
}

fn validate_run_args(args: &[String]) -> FnResult<()> {
    if args.first().is_some_and(|arg| arg == "channel") && args.len() > 1 {
        return Err(plugin_err!(PluginError::Message(format!(
            "{NAME} does not support channel switching with proto. Please use `proto install flutter beta` or check it out with git manually instead. See https://docs.flutter.dev/release/archive#main-channel"
        ))));
    }

    Ok(())
}

fn ensure_supported_host(env: &HostEnvironment) -> FnResult<()> {
    let supported = matches!(
        (env.os, env.arch),
        (HostOS::Linux, HostArch::X64)
            | (HostOS::MacOS, HostArch::X64 | HostArch::Arm64)
            | (HostOS::Windows, HostArch::X64)
    );

    if supported {
        Ok(())
    } else {
        Err(PluginError::UnsupportedOS {
            tool: NAME.to_owned(),
            os: format!("{} ({})", env.os, env.arch),
        }
        .into())
    }
}

fn is_release_compatible(env: &HostEnvironment, release: &FlutterDistVersion) -> bool {
    if !matches!(release.channel.as_str(), "stable" | "beta") {
        return false;
    }

    match env.arch {
        HostArch::Arm64 => release.arch.as_deref() == Some("arm64"),
        HostArch::X64 => matches!(release.arch.as_deref(), None | Some("x64")),
        _ => false,
    }
}

fn build_versions_output(
    env: &HostEnvironment,
    response: &FlutterDist,
) -> FnResult<LoadVersionsOutput> {
    ensure_supported_host(env)?;

    let mut output = LoadVersionsOutput::default();

    for release in &response.releases {
        if !is_release_compatible(env, release) {
            continue;
        }

        let Ok(version_spec) = VersionSpec::parse(&release.version) else {
            continue;
        };
        let Some(version) = version_spec.as_version() else {
            continue;
        };

        if version.major == 0 {
            continue;
        }

        let unresolved = version_spec.to_unresolved_spec();

        if response.latest.stable == release.hash {
            output.latest = Some(unresolved.clone());
            output.aliases.insert("latest".into(), unresolved.clone());
            output.aliases.insert("stable".into(), unresolved.clone());
        }

        if response.latest.beta == release.hash {
            output.aliases.insert("beta".into(), unresolved);
        }

        if !output.versions.contains(&version_spec) {
            output.versions.push(version_spec);
        }
    }

    Ok(output)
}

fn select_release<'a>(
    env: &HostEnvironment,
    response: &'a FlutterDist,
    version_spec: &VersionSpec,
) -> FnResult<&'a FlutterDistVersion> {
    ensure_supported_host(env)?;

    let requested = version_spec.as_version().ok_or_else(|| {
        plugin_err!(PluginError::Message(format!(
            "Unable to resolve {NAME} version `{version_spec}` to an exact release"
        )))
    })?;
    let preferred_channel = if requested.pre.is_empty() {
        "stable"
    } else {
        "beta"
    };

    response
        .releases
        .iter()
        .filter(|release| is_release_compatible(env, release))
        .filter(|release| {
            VersionSpec::parse(&release.version)
                .ok()
                .and_then(|spec| spec.as_version().cloned())
                .is_some_and(|version| version == *requested)
        })
        .min_by_key(|release| usize::from(release.channel != preferred_channel))
        .ok_or_else(|| {
            plugin_err!(PluginError::Message(format!(
                "Unable to install {NAME}@{requested} for {} ({}): no compatible stable or beta archive was found",
                env.os, env.arch
            )))
        })
}

fn build_download_url(base_url: &str, release: &FlutterDistVersion) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        release.archive.trim_start_matches('/')
    )
}

fn get_os_as_str(env: &HostEnvironment) -> &'static str {
    match env.os {
        HostOS::MacOS => "macos",
        HostOS::Windows => "windows",
        _ => "linux",
    }
}

fn fetch_dist(env: &HostEnvironment) -> AnyResult<FlutterDist> {
    let suffix = get_os_as_str(env);
    let base_url = get_tool_config::<FlutterPluginConfig>()?.base_url;
    let url = format!("{base_url}/releases_{suffix}.json");
    let cache_key = format!("dist_{suffix}");

    if let Ok(Some(cached)) = var::get::<Vec<u8>>(&cache_key) {
        if let Ok(dist) = json::from_slice::<FlutterDist>(&cached) {
            return Ok(dist);
        }
    }

    let bytes = fetch_bytes(&url)?;
    let _ = var::set(&cache_key, &bytes);
    let dist: FlutterDist = json::from_slice(&bytes)?;

    Ok(dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> FlutterDist {
        json::from_str(include_str!("../tests/fixtures/releases_macos.json")).unwrap()
    }

    fn host(arch: HostArch) -> HostEnvironment {
        HostEnvironment {
            arch,
            os: HostOS::MacOS,
            ..HostEnvironment::default()
        }
    }

    #[test]
    fn accepts_empty_and_read_only_channel_args() {
        assert!(validate_run_args(&[]).is_ok());
        assert!(validate_run_args(&["doctor".into()]).is_ok());
        assert!(validate_run_args(&["channel".into()]).is_ok());
        assert!(validate_run_args(&["channel".into(), "stable".into()]).is_err());
    }

    #[test]
    fn builds_compatible_versions_and_aliases() {
        let output = build_versions_output(&host(HostArch::Arm64), &fixture()).unwrap();

        assert_eq!(
            output.latest,
            Some(UnresolvedVersionSpec::parse("3.44.8").unwrap())
        );
        assert_eq!(
            output.aliases.get("beta"),
            Some(&UnresolvedVersionSpec::parse("3.47.0-0.2.pre").unwrap())
        );
        assert!(output
            .versions
            .contains(VersionSpec::parse("2.12.0-4.1.pre").unwrap().as_ref()));
        assert!(!output
            .versions
            .contains(VersionSpec::parse("3.48.0-0.1.pre").unwrap().as_ref()));
        assert!(!output
            .versions
            .contains(VersionSpec::parse("0.11.13").unwrap().as_ref()));
    }

    #[test]
    fn selects_archive_metadata_and_preferred_channel() {
        let response = fixture();
        let arm_host = host(HostArch::Arm64);
        let stable =
            select_release(&arm_host, &response, &VersionSpec::parse("3.44.8").unwrap()).unwrap();

        assert_eq!(stable.archive, "stable/macos/custom-stable-package.zip");
        assert_eq!(stable.sha256, "stable-arm64-sha");
        assert_eq!(
            build_download_url("https://example.test/releases/", stable),
            "https://example.test/releases/stable/macos/custom-stable-package.zip"
        );

        let x64_host = host(HostArch::X64);
        let duplicate =
            select_release(&x64_host, &response, &VersionSpec::parse("2.0.0").unwrap()).unwrap();

        assert_eq!(duplicate.channel, "stable");
        assert_eq!(duplicate.sha256, "duplicate-stable-sha");
    }

    #[test]
    fn reports_missing_release_and_unsupported_host() {
        let response = fixture();

        assert!(select_release(
            &host(HostArch::Arm64),
            &response,
            &VersionSpec::parse("1.0.0").unwrap()
        )
        .is_err());

        let windows_x86 = HostEnvironment {
            arch: HostArch::X86,
            os: HostOS::Windows,
            ..HostEnvironment::default()
        };

        assert!(ensure_supported_host(&windows_x86).is_err());
    }
}
