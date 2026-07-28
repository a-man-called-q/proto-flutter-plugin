use proto_pdk_test_utils::*;

mod flutter_tool {
    use super::*;

    #[cfg(feature = "live-tests")]
    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_dist_url() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[cfg(feature = "live-tests")]
    #[tokio::test(flavor = "multi_thread")]
    async fn check_versions_range() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("flutter-test", |config| {
                config.host(HostOS::MacOS, HostArch::X64);
            })
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output
            .versions
            .contains(VersionSpec::parse("3.29.0").unwrap().as_ref()));
        assert!(output
            .versions
            .contains(VersionSpec::parse("1.17.0").unwrap().as_ref()));
        assert!(!output
            .versions
            .contains(VersionSpec::parse("1.12.13").unwrap().as_ref()));
        assert!(output
            .versions
            .contains(VersionSpec::parse("3.30.0-0.1.pre").unwrap().as_ref()));
        assert!(output
            .versions
            .contains(VersionSpec::parse("1.17.0-dev.3.1").unwrap().as_ref()));
        assert!(output
            .versions
            .contains(VersionSpec::parse("1.12.13+hotfix.6").unwrap().as_ref()));
        assert!(!output
            .versions
            .contains(VersionSpec::parse("0.11.13").unwrap().as_ref()));
        assert!(!output
            .versions
            .contains(VersionSpec::parse("1.1.8").unwrap().as_ref()));
        assert!(output.versions.len() >= 245);
    }

    #[cfg(feature = "live-tests")]
    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_stable_beta_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());

        assert!(output.aliases.contains_key("stable"));
        assert_eq!(output.aliases.get("stable"), output.latest.as_ref());

        assert!(output.aliases.contains_key("beta"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_pubspec() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"
name: "My name"
environment:
  flutter: ">=3.29.0"
                    "#
                    .into(),
                    file: "pubspec.yaml".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=3.29.0").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_fvmrc() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "flutter": "3.29.0" }"#.into(),
                    file: ".fvmrc".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("3.29.0").unwrap()),
            }
        );
    }
}
