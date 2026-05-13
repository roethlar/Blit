use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::{Duration, Instant},
};

use eyre::{bail, Context, Result};
use flume::RecvTimeoutError;
use hostname::get;
use log::warn;
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

/// Service type advertised by blit daemons.
pub const BLIT_SERVICE_TYPE: &str = "_blit._tcp.local.";

/// Metadata captured from an mDNS announcement.
#[derive(Debug, Clone)]
pub struct MdnsDiscoveredService {
    pub fullname: String,
    pub instance_name: String,
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<Ipv4Addr>,
    pub properties: HashMap<String, String>,
}

impl MdnsDiscoveredService {
    /// Returns the comma-separated modules advertised by the daemon, if provided.
    pub fn modules(&self) -> Vec<String> {
        self.properties
            .get("modules")
            .map(|value| {
                value
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Module count as advertised in the `module_count` TXT record.
    /// Distinct from `modules().len()` because `modules` is truncated
    /// past ~180 bytes (mDNS TXT size cap); `module_count` always
    /// reflects the true module count even when the modules list is
    /// abbreviated. Returns `None` if the daemon didn't advertise it
    /// (pre-§3.2 daemon).
    pub fn module_count(&self) -> Option<u32> {
        self.properties
            .get("module_count")
            .and_then(|v| v.parse::<u32>().ok())
    }

    /// Whether the daemon accepts `DelegatedPull` requests (remote→
    /// remote initiator). Used by `blit scan` so operators can spot
    /// at a glance which daemons can act as a delegation destination.
    /// Returns `None` if the daemon didn't advertise it (pre-§3.2
    /// daemon).
    pub fn delegation_enabled(&self) -> Option<bool> {
        self.properties
            .get("delegation_enabled")
            .map(|v| matches!(v.as_str(), "1" | "true"))
    }
}

/// Options for advertising a daemon over mDNS.
pub struct AdvertiseOptions<'a> {
    pub port: u16,
    pub instance_name: Option<&'a str>,
    pub module_names: &'a [String],
    /// Whether the daemon accepts DelegatedPull requests. Surfaces
    /// as the `delegation_enabled` TXT record so `blit scan` can
    /// show operators at-a-glance which daemons can act as a
    /// remote→remote delegation destination (§3.2 of
    /// RELEASE_PLAN_v2.1).
    pub delegation_enabled: bool,
}

/// Guard that keeps the mDNS daemon and registration alive for as long as it is held.
pub struct MdnsAdvertiser {
    daemon: Arc<ServiceDaemon>,
    fullname: String,
    instance_name: String,
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        if let Ok(rx) = self.daemon.unregister(&self.fullname) {
            let _ = rx.recv_timeout(Duration::from_secs(1));
        }
        if let Err(err) = self.daemon.shutdown() {
            warn!("failed to shut down mDNS daemon: {err:?}");
        }
    }
}

impl MdnsAdvertiser {
    pub fn fullname(&self) -> &str {
        &self.fullname
    }

    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }
}

/// Starts advertising the daemon using mDNS.
pub fn advertise(options: AdvertiseOptions<'_>) -> Result<MdnsAdvertiser> {
    let daemon = Arc::new(ServiceDaemon::new().context("failed to create mDNS daemon")?);

    // mdns-sd 0.19 enforces a `.local.` suffix on the hostname argument
    // (the old 0.8 was lenient). Append if missing — system hostname is
    // usually the bare label.
    let raw_hostname = get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    let hostname = if raw_hostname.ends_with(".local.") {
        raw_hostname
    } else if raw_hostname.ends_with(".local") {
        format!("{raw_hostname}.")
    } else {
        format!("{raw_hostname}.local.")
    };

    let instance_name = options
        .instance_name
        .map(str::to_owned)
        .unwrap_or_else(default_instance_name);

    let modules_txt = if options.module_names.is_empty() {
        None
    } else {
        Some(truncate_modules(options.module_names))
    };

    let mut properties = HashMap::new();
    properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    // §3.2: module_count is the authoritative count. The `modules`
    // string is human-readable but truncated past ~180 bytes (mDNS
    // TXT size cap), so a daemon exporting hundreds of modules
    // shows "first-few-modules,...(+N more)" in `modules` and the
    // full N+few in `module_count`.
    properties.insert(
        "module_count".to_string(),
        options.module_names.len().to_string(),
    );
    properties.insert(
        "delegation_enabled".to_string(),
        if options.delegation_enabled { "1" } else { "0" }.to_string(),
    );
    if let Some(mods) = modules_txt {
        properties.insert("modules".to_string(), mods);
    }

    let info = ServiceInfo::new(
        BLIT_SERVICE_TYPE,
        &instance_name,
        &hostname,
        (),
        options.port,
        properties,
    )
    .context("failed to build service info")?
    .enable_addr_auto();

    let fullname = info.get_fullname().to_string();
    daemon
        .register(info)
        .with_context(|| format!("failed to register mDNS service '{fullname}'"))?;

    Ok(MdnsAdvertiser {
        daemon,
        fullname,
        instance_name,
    })
}

/// Discovers blit daemons advertised on the local network within `timeout`.
pub fn discover(timeout: Duration) -> Result<Vec<MdnsDiscoveredService>> {
    if timeout.is_zero() {
        bail!("discovery timeout must be greater than zero");
    }

    let daemon = ServiceDaemon::new().context("failed to create mDNS daemon")?;
    let receiver = daemon
        .browse(BLIT_SERVICE_TYPE)
        .context("failed to browse mDNS service")?;

    let mut discovered: HashMap<String, MdnsDiscoveredService> = HashMap::new();
    let start = Instant::now();

    loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            break;
        }
        let remaining = timeout - elapsed;
        let wait_for = remaining.min(Duration::from_millis(200));

        match receiver.recv_timeout(wait_for) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let entry = build_service_entry(&info);
                discovered.insert(entry.fullname.clone(), entry);
            }
            Ok(ServiceEvent::ServiceFound(_, _)) | Ok(ServiceEvent::SearchStarted(_)) => {
                // ignore; we'll handle resolved events
            }
            Ok(ServiceEvent::SearchStopped(_)) => break,
            Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                discovered.remove(&fullname);
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    drop(receiver);
    let _ = daemon.stop_browse(BLIT_SERVICE_TYPE);
    let _ = daemon.shutdown();

    let mut entries: Vec<MdnsDiscoveredService> = discovered.into_values().collect();
    entries.sort_by(|a, b| a.instance_name.cmp(&b.instance_name));
    Ok(entries)
}

fn build_service_entry(info: &ResolvedService) -> MdnsDiscoveredService {
    let properties = info
        .txt_properties
        .iter()
        .map(|prop| (prop.key().to_string(), prop.val_str().to_string()))
        .collect::<HashMap<_, _>>();

    let instance_name = info
        .fullname
        .strip_suffix(BLIT_SERVICE_TYPE)
        .map(|s| s.trim_end_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| info.fullname.clone());

    let addresses = info
        .addresses
        .iter()
        .filter_map(|scoped| match scoped.to_ip_addr() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .collect();

    MdnsDiscoveredService {
        fullname: info.fullname.clone(),
        instance_name,
        hostname: info.host.clone(),
        port: info.port,
        addresses,
        properties,
    }
}

fn truncate_modules(module_names: &[String]) -> String {
    const MAX_LEN: usize = 180;
    let mut joined = String::new();
    for (idx, name) in module_names.iter().enumerate() {
        if !joined.is_empty() {
            joined.push(',');
        }
        joined.push_str(name);
        if joined.len() > MAX_LEN {
            joined.truncate(MAX_LEN);
            joined.push_str("...");
            if idx + 1 < module_names.len() {
                joined.push_str(&format!("(+{} more)", module_names.len() - idx - 1));
            }
            break;
        }
    }
    joined
}

fn default_instance_name() -> String {
    get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .filter(|name| !name.is_empty())
        .map(|hostname| format!("blit@{hostname}"))
        .unwrap_or_else(|| "blit".to_string())
}

#[cfg(test)]
mod accessor_tests {
    //! §3.2: TXT accessor helpers parse the wire shape correctly,
    //! including the legacy case where pre-§3.2 daemons don't
    //! advertise `module_count` / `delegation_enabled`.

    use super::*;

    fn service(props: &[(&str, &str)]) -> MdnsDiscoveredService {
        let mut properties = HashMap::new();
        for (k, v) in props {
            properties.insert((*k).to_string(), (*v).to_string());
        }
        MdnsDiscoveredService {
            fullname: "test._blit._tcp.local.".into(),
            instance_name: "test".into(),
            hostname: "host.local.".into(),
            port: 9031,
            addresses: vec![],
            properties,
        }
    }

    #[test]
    fn module_count_parses_advertised_value() {
        let s = service(&[("module_count", "42")]);
        assert_eq!(s.module_count(), Some(42));
    }

    #[test]
    fn module_count_returns_none_for_pre_v3_2_daemon() {
        let s = service(&[("version", "0.1.0")]);
        assert_eq!(s.module_count(), None);
    }

    #[test]
    fn delegation_enabled_recognizes_truthy_values() {
        assert_eq!(
            service(&[("delegation_enabled", "1")]).delegation_enabled(),
            Some(true)
        );
        assert_eq!(
            service(&[("delegation_enabled", "true")]).delegation_enabled(),
            Some(true)
        );
        assert_eq!(
            service(&[("delegation_enabled", "0")]).delegation_enabled(),
            Some(false)
        );
    }

    #[test]
    fn delegation_enabled_absent_returns_none() {
        let s = service(&[("version", "0.1.0")]);
        assert_eq!(s.delegation_enabled(), None);
    }

    #[test]
    fn modules_accessor_unaffected_by_new_fields() {
        let s = service(&[
            ("modules", "alpha,beta,gamma"),
            ("module_count", "3"),
            ("delegation_enabled", "1"),
        ]);
        assert_eq!(s.modules(), vec!["alpha", "beta", "gamma"]);
        assert_eq!(s.module_count(), Some(3));
        assert_eq!(s.delegation_enabled(), Some(true));
    }
}
