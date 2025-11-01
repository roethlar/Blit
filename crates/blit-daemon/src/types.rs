use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ModuleConfig {
    pub name: String,
    pub path: PathBuf,
    pub read_only: bool,
    pub comment: Option<String>,
    pub use_chroot: bool,
}

#[derive(Debug, Clone)]
pub struct RootExport {
    pub path: PathBuf,
    pub read_only: bool,
    pub use_chroot: bool,
}

#[derive(Debug, Clone)]
pub struct RootSpec {
    pub path: PathBuf,
    pub read_only: bool,
    pub use_chroot: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DeletionStats {
    pub files: u64,
    pub dirs: u64,
}

impl DeletionStats {
    pub fn total(self) -> u64 {
        self.files + self.dirs
    }
}

#[derive(Debug, Clone, Default)]
pub struct MdnsConfig {
    pub disabled: bool,
    pub name: Option<String>,
}

#[derive(Debug)]
pub struct DaemonRuntime {
    pub bind_host: String,
    pub port: u16,
    pub modules: HashMap<String, ModuleConfig>,
    pub default_root: Option<RootExport>,
    pub mdns: MdnsConfig,
    pub motd: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DaemonConfigFile {
    pub daemon: Option<DaemonSection>,
    #[serde(default)]
    pub module: Vec<ModuleSection>,
    #[serde(default)]
    pub root: Option<RootSection>,
    #[serde(default)]
    pub mdns: Option<MdnsSection>,
    #[serde(default)]
    pub motd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DaemonSection {
    pub bind: Option<String>,
    pub port: Option<u16>,
    #[serde(default)]
    pub no_mdns: bool,
    #[serde(default)]
    pub root: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModuleSection {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub use_chroot: bool,
}

#[derive(Debug, Deserialize)]
pub struct RootSection {
    pub path: PathBuf,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub use_chroot: bool,
}

#[derive(Debug, Deserialize)]
pub struct MdnsSection {
    #[serde(default)]
    pub disabled: bool,
    pub name: Option<String>,
}
