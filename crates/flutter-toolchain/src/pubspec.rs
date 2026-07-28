use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum PubDependency {
    #[default]
    Empty,
    Version(String),
    Config(PubDependencyConfig),
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PubDependencyConfig {
    pub path: Option<PathBuf>,
    pub sdk: Option<String>,
    pub version: Option<String>,
    pub git: Option<serde_yml::Value>,
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
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Pubspec {
    pub name: Option<String>,
    pub version: Option<String>,
    pub publish_to: Option<String>,
    pub resolution: Option<String>,
    #[serde(default)]
    pub workspace: Vec<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, PubDependency>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, PubDependency>,
    #[serde(skip)]
    pub path: PathBuf,
}

impl Pubspec {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
        let mut manifest: Self = serde_yml::from_str(&source)
            .map_err(|error| format!("Unable to parse {}: {error}", path.display()))?;
        manifest.path = path.to_path_buf();
        Ok(manifest)
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

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PubLock {
    #[serde(default)]
    pub packages: BTreeMap<String, PubLockPackage>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PubLockPackage {
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub source: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_flutter_and_dart_manifests() {
        let flutter: Pubspec = serde_yml::from_str(
            r#"
name: app
dependencies:
  flutter:
    sdk: flutter
"#,
        )
        .unwrap();
        let dart: Pubspec = serde_yml::from_str(
            r#"
name: core
dependencies:
  collection: ^1.0.0
"#,
        )
        .unwrap();

        assert!(flutter.is_flutter());
        assert!(!dart.is_flutter());
    }
}
