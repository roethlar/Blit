use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, Instant},
};

use eyre::{bail, Context, Result};
use flume::RecvTimeoutError;
use hostname::get;
use log::warn;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

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
}

/// Options for advertising a daemon over mDNS.
pub struct AdvertiseOptions<'a> {
    pub port: u16,
    pub instance_name: Option<&'a str>,
    pub module_names: &'a [String],
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

    let hostname = get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "localhost".to_string());

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
                let entry = build_service_entry(info);
                discovered.insert(entry.fullname.clone(), entry);
            }
            Ok(ServiceEvent::ServiceFound(_, _)) | Ok(ServiceEvent::SearchStarted(_)) => {
                // ignore; we'll handle resolved events
            }
            Ok(ServiceEvent::SearchStopped(_)) => break,
            Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                discovered.remove(&fullname);
            }
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

fn build_service_entry(info: ServiceInfo) -> MdnsDiscoveredService {
    let properties = info
        .get_properties()
        .iter()
        .map(|prop| (prop.key().to_string(), prop.val_str().to_string()))
        .collect::<HashMap<_, _>>();

    let instance_name = info
        .get_fullname()
        .strip_suffix(BLIT_SERVICE_TYPE)
        .map(|s| s.trim_end_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| info.get_fullname().to_string());

    MdnsDiscoveredService {
        fullname: info.get_fullname().to_string(),
        instance_name,
        hostname: info.get_hostname().to_string(),
        port: info.get_port(),
        addresses: info.get_addresses().iter().copied().collect(),
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
