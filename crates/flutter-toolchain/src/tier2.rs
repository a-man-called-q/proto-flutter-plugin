use crate::FlutterToolchainConfig;
use extism_pdk::*;
use flutter_models::{PubDependency, PubLock, Pubspec};
use moon_common::{path::paths_are_equal, Id};
use moon_config::{
    DependencyScope, Input, OneOrMany, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskOptionsConfig, TaskOptionCache, TaskOptionRunInCI,
};
use moon_pdk::{map_miette_error, parse_toolchain_config_schema};
use moon_pdk_api::*;
use moon_target::Target;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct ProjectInfo {
    id: Id,
    root: VirtualPath,
    manifest_path: VirtualPath,
    manifest: Pubspec,
    workspace_root: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum TaskKind {
    Analyze,
    Test,
    Run,
}

struct TaskOptions {
    kind: TaskKind,
    interactive: bool,
    in_workspace: bool,
}

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let config = parse_toolchain_config_schema::<FlutterToolchainConfig>(input.toolchain_config)?;

    if config.version.is_none() {
        return Err(Error::msg(
            "Flutter toolchain version is missing; pin `flutter` in the root `.prototools` or set `flutter.version` in `.moon/toolchains.yml`",
        )
        .into());
    }

    Ok(Json(DefineRequirementsOutput::default()))
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<FlutterToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendProjectGraphOutput::default();
    let mut projects = BTreeMap::<String, ProjectInfo>::new();
    let mut manifests = BTreeMap::<PathBuf, Pubspec>::new();

    for (id, source) in input.project_sources {
        let root = input.context.get_project_root_from_source(&source);
        let manifest_path = root.join("pubspec.yaml");

        if !manifest_path.exists() {
            continue;
        }

        let manifest = load_manifest(&mut manifests, manifest_path.any_path())?;
        let package_name = manifest
            .name
            .clone()
            .ok_or_else(|| Error::msg(format!("{} is missing `name`", manifest_path)))?;
        let workspace_root =
            find_workspace_root(&root, &input.context.workspace_root, &mut manifests)?;

        if projects
            .insert(
                package_name.clone(),
                ProjectInfo {
                    id,
                    root,
                    manifest_path,
                    manifest,
                    workspace_root,
                },
            )
            .is_some()
        {
            return Err(Error::msg(format!(
                "Duplicate Pub package name `{package_name}` in the Moon project graph"
            ))
            .into());
        }
    }

    for project in projects.values() {
        let mut extended = ExtendProjectOutput {
            alias: project.manifest.name.clone(),
            ..Default::default()
        };

        extract_project_dependencies(
            project,
            &projects,
            &project.manifest.dependencies,
            DependencyScope::Production,
            &mut extended,
        );
        extract_project_dependencies(
            project,
            &projects,
            &project.manifest.dev_dependencies,
            DependencyScope::Development,
            &mut extended,
        );

        if config.infer_tasks {
            extended.tasks = infer_tasks(project)?;
        }

        output
            .extended_projects
            .insert(project.id.clone(), extended);

        output.input_files.push(project.manifest_path.to_path_buf());
    }

    Ok(Json(output))
}

fn extract_project_dependencies(
    project: &ProjectInfo,
    projects: &BTreeMap<String, ProjectInfo>,
    dependencies: &BTreeMap<String, PubDependency>,
    scope: DependencyScope,
    output: &mut ExtendProjectOutput,
) {
    for (name, dependency) in dependencies {
        let Some(dep_project) = projects.get(name) else {
            continue;
        };

        let same_workspace = project.workspace_root.is_some()
            && project.workspace_root == dep_project.workspace_root;
        let matching_path = dependency
            .path()
            .is_some_and(|path| paths_are_equal(project.root.join(path), &dep_project.root));

        if (same_workspace || matching_path) && dep_project.id != project.id {
            if scope == DependencyScope::Development
                && output
                    .dependencies
                    .iter()
                    .any(|existing| existing.id == dep_project.id)
            {
                continue;
            }
            output.dependencies.push(ProjectDependency {
                id: dep_project.id.clone(),
                scope,
                via: Some(format!("Pub package {name}")),
            });
        }
    }
}

fn infer_tasks(project: &ProjectInfo) -> AnyResult<BTreeMap<Id, PartialTaskConfig>> {
    let mut tasks = BTreeMap::new();
    let flutter = project.manifest.is_flutter();
    let executable = if flutter { "flutter" } else { "dart" };

    tasks.insert(
        Id::raw("analyze"),
        create_task(
            task_command(executable, "analyze", flutter),
            "Analyze Dart and Flutter source files.",
            "analyze",
            TaskOptions {
                kind: TaskKind::Analyze,
                interactive: false,
                in_workspace: project.workspace_root.is_some(),
            },
        )?,
    );

    let test_dir = project.root.join("test");

    if contains_dart_file(test_dir.any_path()) {
        tasks.insert(
            Id::raw("test"),
            create_task(
                task_command(executable, "test", flutter),
                "Run Dart or Flutter tests.",
                "test",
                TaskOptions {
                    kind: TaskKind::Test,
                    interactive: false,
                    in_workspace: project.workspace_root.is_some(),
                },
            )?,
        );
    }

    if flutter && project.root.join("lib/main.dart").exists() {
        tasks.insert(
            Id::raw("run"),
            create_task(
                vec!["flutter".into(), "run".into(), "--no-pub".into()],
                "Run the Flutter application interactively.",
                "run",
                TaskOptions {
                    kind: TaskKind::Run,
                    interactive: true,
                    in_workspace: project.workspace_root.is_some(),
                },
            )?,
        );
    }

    Ok(tasks)
}

fn task_command(executable: &str, subcommand: &str, flutter: bool) -> Vec<String> {
    let mut command = vec![executable.into(), subcommand.into()];

    if flutter {
        command.push("--no-pub".into());
    }

    command
}

fn create_task(
    command: Vec<String>,
    description: &str,
    upstream_task: &str,
    options: TaskOptions,
) -> AnyResult<PartialTaskConfig> {
    let mut input_patterns = vec!["pubspec.yaml"];

    if options.in_workspace {
        input_patterns.extend([
            "/pubspec.yaml",
            "/pubspec.lock",
            "/analysis_options.{yaml,yml}",
        ]);
    } else {
        input_patterns.extend(["pubspec.lock", "analysis_options.{yaml,yml}"]);
    }

    input_patterns.extend(["lib/**/*.dart", "bin/**/*.dart"]);
    if matches!(options.kind, TaskKind::Test) {
        input_patterns.extend(["test/**/*.dart", "integration_test/**/*.dart"]);
    }
    if matches!(options.kind, TaskKind::Run) {
        input_patterns.extend(["web/**/*", "assets/**/*"]);
    }

    let mut task = PartialTaskConfig {
        command: Some(PartialTaskArgs::List(command)),
        description: Some(description.into()),
        inputs: Some(
            input_patterns
                .into_iter()
                .map(Input::parse)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        toolchains: Some(OneOrMany::One(Id::raw("flutter"))),
        ..Default::default()
    };

    if !options.interactive {
        task.deps = Some(vec![PartialTaskDependency::Target(
            Target::parse(&format!("^:{upstream_task}")).map_err(map_miette_error)?,
        )]);
    } else {
        task.options = Some(PartialTaskOptionsConfig {
            cache: Some(TaskOptionCache::Enabled(false)),
            persistent: Some(true),
            run_in_ci: Some(TaskOptionRunInCI::Enabled(false)),
            ..Default::default()
        });
    }

    Ok(task)
}

const MAX_DART_SEARCH_DEPTH: usize = 64;

fn contains_dart_file(dir: &Path) -> bool {
    contains_dart_file_at_depth(dir, 0)
}

fn contains_dart_file_at_depth(dir: &Path, depth: usize) -> bool {
    if depth >= MAX_DART_SEARCH_DEPTH {
        return false;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            return false;
        };

        if file_type.is_dir() {
            contains_dart_file_at_depth(&path, depth + 1)
        } else {
            file_type.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "dart")
        }
    })
}

fn find_workspace_root(
    project_root: &VirtualPath,
    workspace_root: &VirtualPath,
    manifests: &mut BTreeMap<PathBuf, Pubspec>,
) -> AnyResult<Option<PathBuf>> {
    let mut current = Some(project_root.clone());

    while let Some(dir) = current {
        if !dir.starts_with(workspace_root.any_path()) {
            break;
        }

        let manifest_path = dir.join("pubspec.yaml");

        if manifest_path.exists() {
            let manifest = load_manifest(manifests, manifest_path.any_path())?;

            if manifest.is_workspace_root() {
                let is_member = manifest
                    .workspace
                    .iter()
                    .any(|member| paths_are_equal(dir.join(member), project_root));

                if is_member || paths_are_equal(&dir, project_root) {
                    return Ok(Some(dir.to_path_buf()));
                }
            }
        }

        current = dir.parent();
    }

    Ok(None)
}

fn load_manifest(manifests: &mut BTreeMap<PathBuf, Pubspec>, path: &Path) -> AnyResult<Pubspec> {
    if let Some(manifest) = manifests.get(path) {
        return Ok(manifest.clone());
    }
    let manifest = Pubspec::load(path).map_err(Error::msg)?;
    manifests.insert(path.to_path_buf(), manifest.clone());
    Ok(manifest)
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut current = Some(input.starting_dir.clone());

    while let Some(dir) = current {
        if !dir.starts_with(input.context.workspace_root.any_path()) {
            break;
        }

        let manifest_path = dir.join("pubspec.yaml");

        if manifest_path.exists() {
            let manifest = Pubspec::load(manifest_path.any_path()).map_err(Error::msg)?;

            if manifest.is_workspace_root() {
                validate_workspace_members(dir.any_path(), &manifest)?;

                return Ok(Json(LocateDependenciesRootOutput {
                    root: Some(dir.to_path_buf()),
                    members: Some(manifest.workspace),
                }));
            }
        }

        current = dir.parent();
    }

    let manifest_path = input.starting_dir.join("pubspec.yaml");

    if manifest_path.exists() {
        return Ok(Json(LocateDependenciesRootOutput {
            root: Some(input.starting_dir.to_path_buf()),
            members: None,
        }));
    }

    Ok(Json(LocateDependenciesRootOutput::default()))
}

fn validate_workspace_members(root: &Path, manifest: &Pubspec) -> AnyResult<()> {
    for member in &manifest.workspace {
        let member_manifest = root.join(member).join("pubspec.yaml");

        if !member_manifest.exists() {
            return Err(Error::msg(format!(
                "Pub workspace member `{member}` is missing {}",
                member_manifest.display()
            )));
        }

        let member_pubspec = Pubspec::load(&member_manifest).map_err(Error::msg)?;

        if member_pubspec.resolution.as_deref() != Some("workspace") {
            return Err(Error::msg(format!(
                "Pub workspace member `{member}` must declare `resolution: workspace` in {}",
                member_manifest.display()
            )));
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let manifest = Pubspec::load(input.root.any_path()).map_err(Error::msg)?;
    let executable = if manifest.is_flutter() {
        "flutter"
    } else {
        "dart"
    };

    Ok(Json(InstallDependenciesOutput {
        install_command: Some(
            ExecCommandInput::new(executable, ["pub", "get"])
                .cwd(input.root)
                .into(),
        ),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let manifest = Pubspec::load(input.path.any_path()).map_err(Error::msg)?;
    let mut output = ParseManifestOutput {
        publishable: manifest.publish_to.as_deref() != Some("none"),
        version: manifest
            .version
            .as_deref()
            .and_then(|version| Version::parse(version).ok()),
        ..Default::default()
    };

    output.dependencies = parse_dependencies(&manifest.dependencies)?;
    output.dev_dependencies = parse_dependencies(&manifest.dev_dependencies)?;

    Ok(Json(output))
}

fn parse_dependencies(
    dependencies: &BTreeMap<String, PubDependency>,
) -> AnyResult<BTreeMap<String, ManifestDependency>> {
    let mut output = BTreeMap::new();

    for (name, dependency) in dependencies {
        let parsed = if let Some(path) = dependency.path() {
            ManifestDependency::path(path.to_path_buf())
        } else if let Some((url, git_ref)) = dependency.git() {
            ManifestDependency::Config(ManifestDependencyConfig {
                url: Some(url.into()),
                reference: git_ref.map(Into::into),
                ..Default::default()
            })
        } else if let Some(version) = dependency.version() {
            match UnresolvedVersionSpec::parse(version) {
                Ok(version) => ManifestDependency::Version(version),
                Err(_) => ManifestDependency::Config(ManifestDependencyConfig {
                    reference: Some(version.into()),
                    ..Default::default()
                }),
            }
        } else if dependency.is_flutter_sdk() {
            ManifestDependency::Config(ManifestDependencyConfig {
                reference: Some("sdk:flutter".into()),
                ..Default::default()
            })
        } else {
            ManifestDependency::Inherited(false)
        };

        output.insert(name.clone(), parsed);
    }

    Ok(output)
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let source = std::fs::read_to_string(input.path.any_path())
        .map_err(|error| Error::msg(format!("Unable to read {}: {error}", input.path)))?;
    let lock: PubLock = serde_norway::from_str(&source)
        .map_err(|error| Error::msg(format!("Unable to parse {}: {error}", input.path)))?;
    let mut output = ParseLockOutput::default();

    for (name, package) in lock.packages {
        output
            .dependencies
            .entry(name)
            .or_default()
            .push(LockDependency {
                version: package
                    .version
                    .as_deref()
                    .and_then(|version| VersionSpec::parse(version).ok()),
                hash: package.sha256,
                meta: package.source,
                ..Default::default()
            });
    }

    Ok(Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_creates_test_task_when_tests_exist() {
        let temp =
            std::env::temp_dir().join(format!("flutter-toolchain-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("lib")).unwrap();
        std::fs::write(temp.join("lib/main.dart"), "void main() {}").unwrap();

        let project = ProjectInfo {
            id: Id::raw("app"),
            root: VirtualPath::Real(temp.clone()),
            manifest_path: VirtualPath::Real(temp.join("pubspec.yaml")),
            manifest: serde_norway::from_str(
                "name: app\ndependencies:\n  flutter:\n    sdk: flutter\n",
            )
            .unwrap(),
            workspace_root: None,
        };
        let tasks = infer_tasks(&project).unwrap();

        assert!(tasks.contains_key(&Id::raw("analyze")));
        assert!(tasks.contains_key(&Id::raw("run")));
        assert!(!tasks.contains_key(&Id::raw("test")));

        std::fs::create_dir_all(temp.join("test")).unwrap();
        std::fs::write(temp.join("test/app_test.dart"), "void main() {}").unwrap();
        let tasks = infer_tasks(&project).unwrap();
        assert!(tasks.contains_key(&Id::raw("test")));

        let _ = std::fs::remove_dir_all(temp);
    }
}
