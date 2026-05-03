use clap::Parser;
use eyre::{eyre, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::delegation_gate::{parse_allow_entry, DelegationConfig};

#[derive(Debug, Clone)]
pub(crate) struct ModuleConfig {
    pub(crate) name: String,
    /// Effective filesystem path for this transfer. Mirrors the
    /// historical "module root" but may be mutated by the push
    /// handler to bake in a destination subpath (rsync-style "copy
    /// into here" semantics). Use `canonical_root` for F2
    /// containment checks — `path` may be munged.
    pub(crate) path: PathBuf,
    /// Canonicalized module root from config. Never mutated after
    /// runtime load. F2 containment checks resolve against this so
    /// daemon writes can't escape the original root even when
    /// `path` is rewritten with a destination subpath that contains
    /// (or eventually points through) on-disk symlinks.
    pub(crate) canonical_root: PathBuf,
    pub(crate) read_only: bool,
    pub(crate) _comment: Option<String>,
    /// Per-module narrowing override for delegated-pull
    /// (`[delegation]` master switch). Default true: when the daemon
    /// allows delegation globally, every module participates. Set to
    /// false on a specific module to opt that module out of being a
    /// delegation destination, e.g. for sensitive modules where
    /// operators should always go through the CLI relay.
    ///
    /// The override can only narrow the daemon-wide policy, never
    /// widen it: if `allow_delegated_pull = false` daemon-wide, this
    /// flag has no effect and delegation remains denied.
    pub(crate) delegation_allowed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RootExport {
    pub(crate) path: PathBuf,
    /// Canonicalized form of `path`; see `ModuleConfig::canonical_root`.
    pub(crate) canonical_root: PathBuf,
    pub(crate) read_only: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RootSpec {
    pub(crate) path: PathBuf,
    pub(crate) read_only: bool,
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
    /// Server-side checksums enabled (default: true)
    pub(crate) server_checksums_enabled: bool,
    /// Delegation gate config. `allow_delegated_pull` defaults to
    /// false; the operator must opt the daemon in (and may further
    /// constrain via `allowed_source_hosts`).
    pub(crate) delegation: DelegationConfig,
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
    /// Disable server-side checksum computation (clients will transfer files for local verification)
    #[arg(long)]
    pub(crate) no_server_checksums: bool,
    /// Enable internal RPC counters (push/pull/purge totals, active gauge,
    /// error counter). No exposure mechanism today; reserved for a future
    /// GUI/TUI gRPC `GetState`-style RPC. Off by default — atomic ops are
    /// skipped entirely when disabled.
    #[arg(long)]
    pub(crate) metrics: bool,
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    daemon: RawDaemonSection,
    #[serde(default, rename = "module")]
    modules: Vec<RawModule>,
    #[serde(default)]
    delegation: RawDelegationSection,
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
    no_server_checksums: bool,
}

/// `[delegation]` block from the daemon config. Default: feature off.
/// See `delegation_gate.rs` for matching semantics; entries are parsed
/// (and validated) at config load.
#[derive(Debug, Default, Deserialize)]
struct RawDelegationSection {
    #[serde(default)]
    allow_delegated_pull: bool,
    #[serde(default)]
    allowed_source_hosts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawModule {
    name: String,
    path: PathBuf,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    read_only: bool,
    /// Per-module narrowing override. Defaults to true so existing
    /// configs unaffected. See `ModuleConfig::delegation_allowed`.
    #[serde(default = "default_true")]
    delegation_allowed: bool,
}

fn default_true() -> bool {
    true
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

    // Server checksums: enabled by default, can be disabled via CLI or config
    let server_checksums_enabled = if args.no_server_checksums {
        false
    } else {
        !raw.daemon.no_server_checksums
    };

    // Parse delegation gate config first so an invalid CIDR / bad
    // hostname surfaces as a config-load error before we touch any
    // module paths. This is the §4.3.2 contract: invalid entries fail
    // config load loudly.
    let mut allowed_source_hosts = Vec::with_capacity(raw.delegation.allowed_source_hosts.len());
    for entry in &raw.delegation.allowed_source_hosts {
        let parsed = parse_allow_entry(entry).with_context(|| {
            format!(
                "failed to parse allowed_source_hosts entry '{}' in [delegation]",
                entry
            )
        })?;
        allowed_source_hosts.push(parsed);
    }
    let delegation = DelegationConfig {
        allow_delegated_pull: raw.delegation.allow_delegated_pull,
        allowed_source_hosts,
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
                path: canonical.clone(),
                canonical_root: canonical,
                read_only: module.read_only,
                _comment: module.comment,
                delegation_allowed: module.delegation_allowed,
            },
        );
    }

    let mut root_spec = if let Some(cli_root) = &args.root {
        Some(RootSpec {
            path: cli_root.clone(),
            read_only: false,
        })
    } else if let Some(cfg_root) = raw.daemon.root.clone() {
        Some(RootSpec {
            path: cfg_root,
            read_only: raw.daemon.root_read_only,
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
                canonical_root: canonical.clone(),
                read_only: chosen.read_only,
                _comment: None,
                // Implicit "default" module follows the daemon-wide
                // delegation policy without further narrowing.
                delegation_allowed: true,
            },
        );
        default_root = Some(RootExport {
            path: canonical.clone(),
            canonical_root: canonical,
            read_only: chosen.read_only,
        });
    } else if let Some(spec) = root_spec {
        let canonical = fs::canonicalize(&spec.path).with_context(|| {
            format!(
                "failed to resolve root export path '{}'",
                spec.path.display()
            )
        })?;
        default_root = Some(RootExport {
            path: canonical.clone(),
            canonical_root: canonical,
            read_only: spec.read_only,
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
        server_checksums_enabled,
        delegation,
    })
}

#[cfg(test)]
mod delegation_config_tests {
    //! Tests pinning that bad delegation-gate config fails config
    //! load loudly (Phase 1 unit-test list, R23-F3 contract).
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn with_config(toml: &str) -> (TempDir, DaemonArgs) {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&cfg_path).expect("create config");
        f.write_all(toml.as_bytes()).expect("write config");
        let args = DaemonArgs {
            config: Some(cfg_path),
            bind: None,
            port: None,
            root: Some(dir.path().to_path_buf()),
            no_mdns: true,
            mdns_name: None,
            force_grpc_data: false,
            no_server_checksums: false,
            metrics: false,
        };
        (dir, args)
    }

    #[test]
    fn invalid_cidr_fails_config_load() {
        let toml = r#"
            [delegation]
            allow_delegated_pull = true
            allowed_source_hosts = ["10.0.0.0/99"]
        "#;
        let (_dir, args) = with_config(toml);
        let err = load_runtime(&args).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid CIDR") || msg.contains("allowed_source_hosts"),
            "expected CIDR-related config error, got: {msg}"
        );
    }

    #[test]
    fn empty_allowlist_entry_fails_config_load() {
        let toml = r#"
            [delegation]
            allow_delegated_pull = true
            allowed_source_hosts = ["", "10.0.0.0/8"]
        "#;
        let (_dir, args) = with_config(toml);
        let err = load_runtime(&args).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("empty entry") || msg.contains("allowed_source_hosts"),
            "expected empty-entry rejection, got: {msg}"
        );
    }

    #[test]
    fn valid_cidr_and_hostname_load_cleanly() {
        let toml = r#"
            [delegation]
            allow_delegated_pull = true
            allowed_source_hosts = ["10.0.0.0/8", "server-a.lan", "[::1]"]
        "#;
        let (_dir, args) = with_config(toml);
        let runtime = load_runtime(&args).expect("config loads");
        assert!(runtime.delegation.allow_delegated_pull);
        assert_eq!(runtime.delegation.allowed_source_hosts.len(), 3);
    }

    #[test]
    fn delegation_block_omitted_defaults_to_disabled() {
        // Existing configs without [delegation] still load.
        let toml = "";
        let (_dir, args) = with_config(toml);
        let runtime = load_runtime(&args).expect("default load");
        assert!(!runtime.delegation.allow_delegated_pull);
        assert!(runtime.delegation.allowed_source_hosts.is_empty());
    }

    #[test]
    fn per_module_delegation_allowed_defaults_true() {
        // A module without an explicit `delegation_allowed` setting
        // follows the daemon-wide policy without further narrowing.
        let dir = tempfile::tempdir().expect("tempdir");
        let mod_path = dir.path().join("mod1");
        std::fs::create_dir_all(&mod_path).expect("create module dir");
        let cfg_path = dir.path().join("config.toml");
        let toml = format!(
            r#"
                [[module]]
                name = "alpha"
                path = {path:?}
            "#,
            path = mod_path.canonicalize().unwrap().to_str().unwrap()
        );
        std::fs::write(&cfg_path, toml).expect("write config");
        let args = DaemonArgs {
            config: Some(cfg_path),
            bind: None,
            port: None,
            root: None,
            no_mdns: true,
            mdns_name: None,
            force_grpc_data: false,
            no_server_checksums: false,
            metrics: false,
        };
        let runtime = load_runtime(&args).expect("config loads");
        assert!(runtime.modules["alpha"].delegation_allowed);
    }

    #[test]
    fn per_module_delegation_allowed_can_opt_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mod_path = dir.path().join("mod1");
        std::fs::create_dir_all(&mod_path).expect("create module dir");
        let cfg_path = dir.path().join("config.toml");
        let toml = format!(
            r#"
                [[module]]
                name = "alpha"
                path = {path:?}
                delegation_allowed = false
            "#,
            path = mod_path.canonicalize().unwrap().to_str().unwrap()
        );
        std::fs::write(&cfg_path, toml).expect("write config");
        let args = DaemonArgs {
            config: Some(cfg_path),
            bind: None,
            port: None,
            root: None,
            no_mdns: true,
            mdns_name: None,
            force_grpc_data: false,
            no_server_checksums: false,
            metrics: false,
        };
        let runtime = load_runtime(&args).expect("config loads");
        assert!(!runtime.modules["alpha"].delegation_allowed);
    }
}
