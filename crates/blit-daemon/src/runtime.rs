use clap::Parser;
use eyre::{eyre, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct ModuleConfig {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) read_only: bool,
    pub(crate) _comment: Option<String>,
    pub(crate) _use_chroot: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RootExport {
    pub(crate) path: PathBuf,
    pub(crate) read_only: bool,
    pub(crate) use_chroot: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RootSpec {
    pub(crate) path: PathBuf,
    pub(crate) read_only: bool,
    pub(crate) use_chroot: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MdnsConfig {
    pub(crate) disabled: bool,
    pub(crate) name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DaemonRuntime {
    pub(crate) bind_host: String,
    pub(crate) port: u16,
    pub(crate) modules: HashMap<String, ModuleConfig>,
    pub(crate) default_root: Option<RootExport>,
    pub(crate) mdns: MdnsConfig,
    pub(crate) motd: Option<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "blit-daemon", about = "Remote transfer daemon for blit v2")]
pub(crate) struct DaemonArgs {
    /// Path to the daemon configuration file (TOML). Defaults to /etc/blit/config.toml when present.
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,
    /// Host/IP address to bind (overrides config file)
    #[arg(long)]
    pub(crate) bind: Option<String>,
    /// Port to bind (overrides config file)
    #[arg(long)]
    pub(crate) port: Option<u16>,
    /// Exported root path for server:// when no modules are defined
    #[arg(long)]
    pub(crate) root: Option<PathBuf>,
    /// Disable mDNS advertisement even if enabled in config
    #[arg(long)]
    pub(crate) no_mdns: bool,
    /// Override the advertised mDNS instance name
    #[arg(long)]
    pub(crate) mdns_name: Option<String>,
    /// Force the daemon to use the gRPC data plane instead of TCP
    #[arg(long)]
    pub(crate) force_grpc_data: bool,
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    daemon: RawDaemonSection,
    #[serde(default, rename = "module")]
    modules: Vec<RawModule>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDaemonSection {
    bind: Option<String>,
    port: Option<u16>,
    motd: Option<String>,
    no_mdns: Option<bool>,
    mdns_name: Option<String>,
    root: Option<PathBuf>,
    #[serde(default)]
    root_read_only: bool,
    #[serde(default)]
    root_use_chroot: bool,
}

#[derive(Debug, Deserialize)]
struct RawModule {
    name: String,
    path: PathBuf,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    use_chroot: bool,
}

fn default_config_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\Blit\config.toml")
    } else {
        PathBuf::from("/etc/blit/config.toml")
    }
}

pub(crate) fn load_runtime(args: &DaemonArgs) -> Result<DaemonRuntime> {
    let mut warnings = Vec::new();

    let config_path = if let Some(path) = &args.config {
        Some(path.clone())
    } else {
        let candidate = default_config_path();
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    };

    let raw = if let Some(ref path) = config_path {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        toml::from_str::<RawConfig>(&contents)
            .with_context(|| format!("failed to parse config file {}", path.display()))?
    } else {
        RawConfig::default()
    };

    let bind_host = args
        .bind
        .clone()
        .or_else(|| raw.daemon.bind.clone())
        .unwrap_or_else(|| "0.0.0.0".to_string());
    let port = args.port.or(raw.daemon.port).unwrap_or(9031);

    let motd = raw.daemon.motd.clone();
    let mdns_disabled = if args.no_mdns {
        true
    } else {
        raw.daemon.no_mdns.unwrap_or(false)
    };
    let mdns_name = args.mdns_name.clone().or(raw.daemon.mdns_name.clone());
    let mdns = MdnsConfig {
        disabled: mdns_disabled,
        name: mdns_name,
    };

    let mut modules = HashMap::new();
    for module in raw.modules {
        if module.name.trim().is_empty() {
            return Err(eyre!("module names cannot be empty"));
        }
        if modules.contains_key(&module.name) {
            return Err(eyre!("duplicate module '{}' in config", module.name));
        }
        let canonical = fs::canonicalize(&module.path).with_context(|| {
            format!(
                "failed to resolve path '{}' for module '{}'",
                module.path.display(),
                module.name
            )
        })?;
        modules.insert(
            module.name.clone(),
            ModuleConfig {
                name: module.name,
                path: canonical,
                read_only: module.read_only,
                _comment: module.comment,
                _use_chroot: module.use_chroot,
            },
        );
    }

    let mut root_spec = if let Some(cli_root) = &args.root {
        Some(RootSpec {
            path: cli_root.clone(),
            read_only: false,
            use_chroot: raw.daemon.root_use_chroot,
        })
    } else if let Some(cfg_root) = raw.daemon.root.clone() {
        Some(RootSpec {
            path: cfg_root,
            read_only: raw.daemon.root_read_only,
            use_chroot: raw.daemon.root_use_chroot,
        })
    } else {
        None
    };

    let mut default_root = None;

    if modules.is_empty() {
        let chosen = if let Some(spec) = root_spec.take() {
            spec
        } else {
            let cwd = std::env::current_dir().context("failed to determine working directory")?;
            warnings.push(format!(
                "no modules configured; exporting working directory {} as 'default'",
                cwd.display()
            ));
            RootSpec {
                path: cwd,
                read_only: false,
                use_chroot: false,
            }
        };
        let canonical = fs::canonicalize(&chosen.path).with_context(|| {
            format!(
                "failed to resolve default export path '{}'",
                chosen.path.display()
            )
        })?;
        modules.insert(
            "default".to_string(),
            ModuleConfig {
                name: "default".to_string(),
                path: canonical.clone(),
                read_only: chosen.read_only,
                _comment: None,
                _use_chroot: chosen.use_chroot,
            },
        );
        default_root = Some(RootExport {
            path: canonical,
            read_only: chosen.read_only,
            use_chroot: chosen.use_chroot,
        });
    } else if let Some(spec) = root_spec {
        let canonical = fs::canonicalize(&spec.path).with_context(|| {
            format!(
                "failed to resolve root export path '{}'",
                spec.path.display()
            )
        })?;
        default_root = Some(RootExport {
            path: canonical,
            read_only: spec.read_only,
            use_chroot: spec.use_chroot,
        });
    } else if !modules.contains_key("default") {
        warnings.push(
            "no default root configured; server:// requests will be rejected until --root or config root is provided"
                .to_string(),
        );
    }

    Ok(DaemonRuntime {
        bind_host,
        port,
        modules,
        default_root,
        mdns,
        motd,
        warnings,
    })
}
