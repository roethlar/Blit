use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use eyre::{Context, Result};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DaemonConfig {
    /// Bind address for the gRPC server
    pub bind_address: Option<String>,
    /// Port for the gRPC server
    pub port: Option<u16>,
    /// Message of the day
    pub motd: Option<String>,
    /// Disable mDNS advertisement
    #[serde(default)]
    pub no_mdns: bool,
    /// Custom mDNS service name
    pub mdns_name: Option<String>,
    /// Disable server-side checksum computation (clients will transfer files for local verification)
    #[serde(default)]
    pub no_server_checksums: bool,
    /// Module definitions
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleConfig {
    /// Path to the module root
    pub path: PathBuf,
    /// Read-only access
    #[serde(default)]
    pub read_only: bool,
    /// Comment/description
    pub comment: Option<String>,
}

impl DaemonConfig {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .await
            .wrap_err_with(|| format!("failed to read config file: {}", path.display()))?;
        
        let config: DaemonConfig = toml::from_str(&content)
            .wrap_err("failed to parse config file")?;
            
        Ok(config)
    }
}
