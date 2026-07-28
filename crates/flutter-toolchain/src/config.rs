use moon_pdk_api::{config_struct, UnresolvedVersionSpec};
use schematic::Config;

config_struct!(
    /// Configures Flutter and Dart support for Moon projects.
    #[derive(Config)]
    pub struct FlutterToolchainConfig {
        /// Automatically infer analyze, test, and application run tasks.
        #[setting(default = true)]
        pub infer_tasks: bool,

        /// Flutter version managed by Proto. This is normally inherited from
        /// the root `.prototools` through `versionFromPrototools`.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
