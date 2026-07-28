use moon_common::Id;
use moon_config::{DependencyScope, LanguageType};
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn registers_flutter_toolchain_metadata() {
    let sandbox = create_empty_moon_sandbox();
    let plugin = sandbox.create_toolchain("flutter").await;
    let output = plugin
        .register_toolchain(RegisterToolchainInput {
            id: Id::raw("flutter"),
        })
        .await;

    assert_eq!(output.name, "Flutter");
    assert_eq!(output.language, Some(LanguageType::other("dart").unwrap()));
    assert_eq!(output.vendor_dir_name.as_deref(), Some(".dart_tool"));
    assert_eq!(output.exe_names, ["flutter", "dart"]);
    assert_eq!(output.manifest_file_names, ["pubspec.yaml"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn infers_pub_workspace_graph_and_tasks() {
    let sandbox = create_moon_sandbox("workspace");
    let plugin = sandbox.create_toolchain("flutter").await;
    let mut input = ExtendProjectGraphInput {
        toolchain_config: json!({
            "inferTasks": true,
            "version": "3.44.8"
        }),
        ..Default::default()
    };
    input
        .project_sources
        .insert(Id::raw("app"), "apps/app".into());
    input
        .project_sources
        .insert(Id::raw("core"), "packages/core".into());

    let output = plugin.extend_project_graph(input).await;
    let app = output.extended_projects.get(&Id::raw("app")).unwrap();
    let core = output.extended_projects.get(&Id::raw("core")).unwrap();

    assert_eq!(app.alias.as_deref(), Some("example_app"));
    assert_eq!(core.alias.as_deref(), Some("example_core"));
    assert_eq!(app.dependencies.len(), 1);
    assert_eq!(app.dependencies[0].id, Id::raw("core"));
    assert_eq!(app.dependencies[0].scope, DependencyScope::Production);
    assert!(app.tasks.contains_key(&Id::raw("analyze")));
    assert!(app.tasks.contains_key(&Id::raw("test")));
    assert!(app.tasks.contains_key(&Id::raw("run")));
    assert!(core.tasks.contains_key(&Id::raw("analyze")));
    assert!(!core.tasks.contains_key(&Id::raw("test")));
    assert!(!core.tasks.contains_key(&Id::raw("run")));

    let mut disabled_input = ExtendProjectGraphInput {
        toolchain_config: json!({
            "inferTasks": false,
            "version": "3.44.8"
        }),
        ..Default::default()
    };
    disabled_input
        .project_sources
        .insert(Id::raw("app"), "apps/app".into());
    let disabled = plugin.extend_project_graph(disabled_input).await;
    assert!(disabled.extended_projects[&Id::raw("app")].tasks.is_empty());
}
