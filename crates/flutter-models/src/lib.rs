use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlutterDistVersion {
    pub archive: String,
    pub hash: String,
    pub channel: String,
    pub version: String,
    pub sha256: String,
    #[serde(rename(deserialize = "dart_sdk_arch"))]
    pub arch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlutterDistLatest {
    pub stable: String,
    pub beta: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlutterDist {
    #[serde(rename(deserialize = "current_release"))]
    pub latest: FlutterDistLatest,
    pub releases: Vec<FlutterDistVersion>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PubDependency {
    #[default]
    Empty,
    Version(String),
    Config(PubDependencyConfig),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PubDependencyConfig {
    pub path: Option<PathBuf>,
    pub sdk: Option<String>,
    pub version: Option<String>,
    pub git: Option<PubGitDependency>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PubGitDependency {
    Url(String),
    Config {
        url: String,
        #[serde(rename = "ref")]
        git_ref: Option<String>,
    },
}

impl PubDependency {
    pub fn is_flutter_sdk(&self) -> bool {
        matches!(self, Self::Config(config) if config.sdk.as_deref() == Some("flutter"))
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Config(config) => config.path.as_deref(),
            _ => None,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Version(version) => Some(version),
            Self::Config(config) => config.version.as_deref(),
            Self::Empty => None,
        }
    }

    pub fn git(&self) -> Option<(&str, Option<&str>)> {
        match self {
            Self::Config(PubDependencyConfig {
                git: Some(PubGitDependency::Url(url)),
                ..
            }) => Some((url, None)),
            Self::Config(PubDependencyConfig {
                git: Some(PubGitDependency::Config { url, git_ref }),
                ..
            }) => Some((url, git_ref.as_deref())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PubspecEnvironment {
    pub flutter: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Pubspec {
    pub name: Option<String>,
    pub version: Option<String>,
    pub publish_to: Option<String>,
    pub resolution: Option<String>,
    pub environment: Option<PubspecEnvironment>,
    #[serde(default)]
    pub workspace: Vec<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, PubDependency>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, PubDependency>,
}

impl Pubspec {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
        serde_norway::from_str(&source)
            .map_err(|error| format!("Unable to parse {}: {error}", path.display()))
    }
    pub fn is_flutter(&self) -> bool {
        self.dependencies
            .get("flutter")
            .is_some_and(PubDependency::is_flutter_sdk)
            || self
                .dev_dependencies
                .get("flutter_test")
                .is_some_and(PubDependency::is_flutter_sdk)
    }
    pub fn is_workspace_root(&self) -> bool {
        !self.workspace.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PubLock {
    #[serde(default)]
    pub packages: BTreeMap<String, PubLockPackage>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PubLockPackage {
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub source: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Fvmrc {
    pub flutter: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pubspec_and_git_dependency() {
        let manifest: Pubspec = serde_norway::from_str(
            "name: app\nenvironment:\n  flutter: ^3.0.0\ndependencies:\n  core:\n    git:\n      url: https://github.com/example/core.git\n      ref: main\n",
        )
        .unwrap();

        assert_eq!(
            manifest.environment.unwrap().flutter.as_deref(),
            Some("^3.0.0")
        );
        assert_eq!(
            manifest.dependencies["core"].git(),
            Some(("https://github.com/example/core.git", Some("main")))
        );
    }

    #[test]
    fn classifies_flutter_manifest() {
        let manifest: Pubspec =
            serde_norway::from_str("dependencies:\n  flutter:\n    sdk: flutter\n").unwrap();
        assert!(manifest.is_flutter());
    }
}
