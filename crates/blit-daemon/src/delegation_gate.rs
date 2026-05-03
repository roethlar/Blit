//! Delegation gate for the `DelegatedPull` RPC.
//!
//! Adding `DelegatedPull` gives any caller that can reach this daemon's
//! control plane the ability to make this daemon initiate an outbound
//! TCP connection to a source endpoint **chosen by the caller**. That
//! is a new outbound-network capability — an SSRF/network-pivot
//! primitive — that did not exist before delegation. The gate exists
//! to make this capability operator-controlled and opt-in.
//!
//! See `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` §4.3 for the full
//! design rationale, ordering invariants, and matching semantics.
//!
//! # Ordering invariant
//!
//! The handler invokes the gate **before** any module path resolution
//! and **before** any outbound connect. The handler-side ordering is
//! documented at the call site; the gate itself does not perform any
//! filesystem I/O or outbound TCP — only DNS resolution.
//!
//! # Matching semantics (§4.3.3)
//!
//! - **Hostname normalization**: case-insensitive, trailing dot
//!   stripped, IDNA punycode normalization for non-ASCII names.
//! - **CIDR / bare IP**: parsed once at config load; invalid entries
//!   fail config load loudly.
//! - **Hostname allowlist matches**: exact post-normalization equality.
//!   No wildcards in 0.1.0.
//! - **Resolution**: the source hostname (or literal IP) is resolved
//!   once. **Every** resolved address must match either a CIDR entry
//!   or a bare-IP entry; a hostname-form entry is sufficient on its
//!   own only for non-special-range addresses.
//! - **Loopback / link-local / unique-local / unspecified ranges
//!   require IP- or CIDR-form authorization** (R25-F3). Hostname-only
//!   entries cannot authorize them. Closes the SSRF-via-DNS pivot
//!   where an attacker-controlled DNS record points an allowlisted
//!   hostname at the daemon's loopback.
//! - **DNS-rebinding mitigation**: the resolved IP is bound to the
//!   outbound connection. The handler connects to a specific
//!   `SocketAddr`, not a re-resolvable hostname.

use std::net::{IpAddr, SocketAddr};

use eyre::{bail, Result};
use ipnet::IpNet;

/// One entry in `[delegation].allowed_source_hosts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AllowEntry {
    /// A normalized hostname. Matches any resolved IP for this name
    /// **as long as the resolved IP is not in a special range** (see
    /// `is_special_range`); for special-range IPs the operator must
    /// also list a CIDR or bare-IP entry covering that address.
    Hostname(String),
    /// A CIDR block. Matches any resolved IP that falls inside it.
    Cidr(IpNet),
    /// A literal IP address. Matches an identical resolved address.
    BareIp(IpAddr),
}

/// Parsed delegation configuration. Built once at daemon config load
/// and used per-RPC by the handler.
#[derive(Debug, Clone, Default)]
pub(crate) struct DelegationConfig {
    /// Master switch. False by default — delegation is opt-in.
    pub(crate) allow_delegated_pull: bool,
    /// Source allowlist. Empty + `allow_delegated_pull = true` means
    /// "any host," which is acceptable on a fully trusted LAN but
    /// must be a deliberate operator choice.
    pub(crate) allowed_source_hosts: Vec<AllowEntry>,
}

/// Why the gate denied a request. Surfaced verbatim to the CLI as the
/// `upstream_message` of a `DelegatedPullError{phase=DELEGATION_REJECTED}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateDenial {
    /// The daemon-wide master switch is off.
    MasterSwitchOff,
    /// The locator's host failed parsing or normalization.
    InvalidHost(String),
    /// DNS resolution of the host failed or returned no addresses.
    UnresolvableHost(String),
    /// One or more resolved addresses are not covered by any allowlist
    /// entry. The reason string lists which addresses failed.
    UnauthorizedAddress(String),
    /// A resolved address is in a special range (loopback / link-local
    /// / unique-local / unspecified) and only a hostname-form entry
    /// authorizes it. Per §4.3.3 rule 7, special ranges require an
    /// IP- or CIDR-form entry.
    SpecialRangeNeedsIpAuth(String),
    /// The per-module override denied delegation against this module
    /// even though the daemon-wide policy permits it. Populated from
    /// the handler when `ModuleConfig::delegation_allowed` is false.
    #[allow(dead_code)]
    ModuleOptOut { module: String },
}

impl GateDenial {
    pub(crate) fn reason(&self) -> String {
        match self {
            GateDenial::MasterSwitchOff => {
                "delegated pull is disabled on this daemon (set [delegation] \
                 allow_delegated_pull = true to enable)"
                    .to_string()
            }
            GateDenial::InvalidHost(host) => {
                format!("invalid source host '{host}'")
            }
            GateDenial::UnresolvableHost(host) => {
                format!("could not resolve source host '{host}'")
            }
            GateDenial::UnauthorizedAddress(detail) => {
                format!("source not in allowlist: {detail}")
            }
            GateDenial::SpecialRangeNeedsIpAuth(detail) => {
                format!(
                    "source resolves to a special-range address that requires \
                     IP- or CIDR-form authorization (not a hostname entry): {detail}"
                )
            }
            GateDenial::ModuleOptOut { module } => {
                format!("module '{module}' opts out of being a delegation destination")
            }
        }
    }
}

/// Parse one allowlist entry from raw config text. Tries CIDR first
/// (because `192.168.1.0/24` parses as a hostname under the IDNA
/// liberal policy), then bare IP, then falls back to hostname.
///
/// Hostnames are normalized: lowercased, trailing dot stripped, IDNA
/// punycode normalization applied. Returns a clear error string on
/// failure (so config-load errors point at the offending entry).
pub(crate) fn parse_allow_entry(raw: &str) -> Result<AllowEntry> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("empty entry in allowed_source_hosts");
    }

    // Try CIDR first — it's the only form containing '/'.
    if trimmed.contains('/') {
        let net: IpNet = trimmed
            .parse()
            .map_err(|e| eyre::eyre!("invalid CIDR '{trimmed}': {e}"))?;
        return Ok(AllowEntry::Cidr(net));
    }

    // Strip brackets for IPv6 literal forms like "[::1]".
    let unbracketed = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    if let Ok(ip) = unbracketed.parse::<IpAddr>() {
        return Ok(AllowEntry::BareIp(normalize_ip(ip)));
    }

    // Fall through to hostname.
    let hostname = normalize_hostname(trimmed)
        .map_err(|e| eyre::eyre!("invalid hostname '{trimmed}': {e}"))?;
    Ok(AllowEntry::Hostname(hostname))
}

/// Normalize a hostname per §4.3.3 rule 1: lowercase, strip trailing
/// dot, apply IDNA punycode. Returns the ASCII (punycode) form.
pub(crate) fn normalize_hostname(raw: &str) -> std::result::Result<String, String> {
    let mut s = raw.trim();
    // Strip a single trailing dot — `server.lan.` and `server.lan`
    // are equivalent.
    if let Some(stripped) = s.strip_suffix('.') {
        s = stripped;
    }
    if s.is_empty() {
        return Err("hostname is empty after normalization".to_string());
    }
    // IDNA conversion to ASCII (punycode). `idna::domain_to_ascii_strict`
    // rejects names that would round-trip differently and gives us a
    // stable comparison form.
    let ascii = idna::domain_to_ascii(s).map_err(|e| format!("IDNA failure: {e}"))?;
    Ok(ascii.to_ascii_lowercase())
}

/// Normalize an IP for comparison: flatten IPv4-mapped IPv6 to its
/// IPv4 form (per §4.3.3 rule 6).
pub(crate) fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    }
}

/// Special ranges that require IP- or CIDR-form authorization (R25-F3).
/// Hostname-form allowlist entries cannot authorize these even if the
/// hostname matches.
pub(crate) fn is_special_range(ip: IpAddr) -> bool {
    let normalized = normalize_ip(ip);
    match normalized {
        IpAddr::V4(v4) => {
            v4.is_loopback()              // 127.0.0.0/8
                || v4.is_link_local()     // 169.254.0.0/16
                || v4.is_unspecified()    // 0.0.0.0
                || v4.octets()[0] == 0    // 0.0.0.0/8 ("this network")
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()              // ::1
                || v6.is_unspecified()    // ::
                || v6.segments()[0] & 0xffc0 == 0xfe80   // fe80::/10 link-local
                || v6.segments()[0] & 0xfe00 == 0xfc00   // fc00::/7 unique-local
        }
    }
}

/// True iff the resolved IP is covered by some IP-form or CIDR-form
/// entry in the allowlist. Hostname entries don't count.
fn matched_by_ip_form(ip: IpAddr, entries: &[AllowEntry]) -> bool {
    let normalized = normalize_ip(ip);
    entries.iter().any(|e| match e {
        AllowEntry::Hostname(_) => false,
        AllowEntry::Cidr(net) => net.contains(&normalized),
        AllowEntry::BareIp(literal) => *literal == normalized,
    })
}

/// True iff the (post-normalization) hostname matches some hostname
/// entry in the allowlist.
fn matched_by_hostname(hostname: &str, entries: &[AllowEntry]) -> bool {
    entries.iter().any(|e| match e {
        AllowEntry::Hostname(h) => h == hostname,
        _ => false,
    })
}

/// The locator the gate inspects. Decoupled from the proto type so
/// tests can construct one directly without touching tonic.
#[derive(Debug, Clone)]
pub(crate) struct LocatorView<'a> {
    pub(crate) host: &'a str,
    pub(crate) port: u16,
}

/// Resolver trait so tests can simulate DNS rebinding (resolver
/// returns A on the first call, B on the second).
#[async_trait::async_trait]
pub(crate) trait HostResolver: Send + Sync {
    async fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<IpAddr>>;
}

/// Production resolver — uses tokio's stdlib-backed lookup. The port
/// is folded in only because `lookup_host` requires a `host:port`
/// string; the returned IPs are what the gate validates.
pub(crate) struct StdResolver;

#[async_trait::async_trait]
impl HostResolver for StdResolver {
    async fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<IpAddr>> {
        let addrs = tokio::net::lookup_host((host, port)).await?;
        Ok(addrs.map(|sa| sa.ip()).collect())
    }
}

/// Validate the locator against the gate config, resolving the host
/// once and binding the outbound connection to the chosen IP. Returns
/// the `SocketAddr` the handler should connect to (resolved-IP +
/// port), or a `GateDenial` describing why delegation was refused.
pub(crate) async fn validate_source<R: HostResolver + ?Sized>(
    config: &DelegationConfig,
    locator: &LocatorView<'_>,
    resolver: &R,
) -> std::result::Result<SocketAddr, GateDenial> {
    if !config.allow_delegated_pull {
        return Err(GateDenial::MasterSwitchOff);
    }

    if locator.host.trim().is_empty() {
        return Err(GateDenial::InvalidHost(locator.host.to_string()));
    }

    // Locator's host can be a literal IP (with or without brackets) or
    // a hostname. Normalize either way.
    let unbracketed = locator
        .host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(locator.host);

    let (normalized_hostname, literal_ip): (Option<String>, Option<IpAddr>) =
        if let Ok(ip) = unbracketed.parse::<IpAddr>() {
            (None, Some(normalize_ip(ip)))
        } else {
            match normalize_hostname(unbracketed) {
                Ok(h) => (Some(h), None),
                Err(_) => return Err(GateDenial::InvalidHost(locator.host.to_string())),
            }
        };

    // Resolution. For a literal IP we skip DNS entirely.
    let resolved: Vec<IpAddr> = if let Some(ip) = literal_ip {
        vec![ip]
    } else {
        let host = normalized_hostname
            .as_deref()
            .expect("hostname or IP must be set");
        match resolver.resolve(host, locator.port).await {
            Ok(addrs) if !addrs.is_empty() => addrs.into_iter().map(normalize_ip).collect(),
            Ok(_) => return Err(GateDenial::UnresolvableHost(host.to_string())),
            Err(_) => return Err(GateDenial::UnresolvableHost(host.to_string())),
        }
    };

    // If the allowlist is empty AND the master switch is on, the
    // operator has explicitly opted into "any host" — accept the
    // first resolved address.
    if config.allowed_source_hosts.is_empty() {
        let first = resolved
            .into_iter()
            .next()
            .ok_or_else(|| GateDenial::UnresolvableHost(unbracketed.to_string()))?;
        return Ok(SocketAddr::new(first, locator.port));
    }

    // For each resolved address, decide whether the allowlist
    // authorizes it:
    //
    //   * if the address is in a special range, ONLY an IP- or
    //     CIDR-form entry can authorize it (R25-F3);
    //   * otherwise, either an IP/CIDR match OR (for a name-based
    //     locator) a hostname entry that matches the normalized
    //     hostname is enough.
    let hostname_matches = match &normalized_hostname {
        Some(h) => matched_by_hostname(h, &config.allowed_source_hosts),
        None => false,
    };

    let mut chosen: Option<IpAddr> = None;
    for ip in &resolved {
        let by_ip = matched_by_ip_form(*ip, &config.allowed_source_hosts);
        if is_special_range(*ip) && !by_ip {
            return Err(GateDenial::SpecialRangeNeedsIpAuth(format!(
                "{} resolved to {} which is in a special range and was not \
                 authorized by an IP/CIDR entry (hostname entries do not \
                 authorize special ranges)",
                normalized_hostname
                    .as_deref()
                    .unwrap_or(unbracketed),
                ip
            )));
        }
        if !by_ip && !hostname_matches {
            return Err(GateDenial::UnauthorizedAddress(format!(
                "{} resolved to {} which matches no allowlist entry",
                normalized_hostname
                    .as_deref()
                    .unwrap_or(unbracketed),
                ip
            )));
        }
        if chosen.is_none() {
            chosen = Some(*ip);
        }
    }

    let ip = chosen.ok_or_else(|| GateDenial::UnresolvableHost(unbracketed.to_string()))?;
    Ok(SocketAddr::new(ip, locator.port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::sync::Mutex;

    /// Test resolver that returns scripted responses, advancing the
    /// script on each call. Used for the DNS-rebinding test.
    struct ScriptedResolver {
        script: Mutex<Vec<Vec<IpAddr>>>,
    }

    impl ScriptedResolver {
        fn new(responses: Vec<Vec<IpAddr>>) -> Self {
            Self {
                script: Mutex::new(responses),
            }
        }
    }

    #[async_trait::async_trait]
    impl HostResolver for ScriptedResolver {
        async fn resolve(&self, _host: &str, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            let mut s = self.script.lock().unwrap();
            if s.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "ScriptedResolver exhausted",
                ));
            }
            Ok(s.remove(0))
        }
    }

    fn ip4(s: &str) -> IpAddr {
        IpAddr::V4(s.parse::<Ipv4Addr>().unwrap())
    }

    // ── parse_allow_entry ────────────────────────────────────────────

    #[test]
    fn parse_hostname_entry_lowercases_and_strips_trailing_dot() {
        let entry = parse_allow_entry("Server-A.LAN.").unwrap();
        assert_eq!(entry, AllowEntry::Hostname("server-a.lan".to_string()));
    }

    #[test]
    fn parse_cidr_v4() {
        let entry = parse_allow_entry("10.0.0.0/8").unwrap();
        match entry {
            AllowEntry::Cidr(net) => assert_eq!(net.to_string(), "10.0.0.0/8"),
            _ => panic!("expected Cidr"),
        }
    }

    #[test]
    fn parse_cidr_v6() {
        let entry = parse_allow_entry("fd00::/8").unwrap();
        assert!(matches!(entry, AllowEntry::Cidr(_)));
    }

    #[test]
    fn parse_bare_ipv4() {
        let entry = parse_allow_entry("127.0.0.1").unwrap();
        assert_eq!(entry, AllowEntry::BareIp(ip4("127.0.0.1")));
    }

    #[test]
    fn parse_bare_ipv6() {
        let entry = parse_allow_entry("::1").unwrap();
        assert!(matches!(entry, AllowEntry::BareIp(IpAddr::V6(_))));
    }

    #[test]
    fn parse_bracketed_ipv6() {
        let entry = parse_allow_entry("[::1]").unwrap();
        assert!(matches!(entry, AllowEntry::BareIp(IpAddr::V6(_))));
    }

    #[test]
    fn parse_invalid_cidr_fails() {
        let err = parse_allow_entry("10.0.0.0/99").unwrap_err();
        assert!(err.to_string().contains("invalid CIDR"));
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse_allow_entry("   ").is_err());
    }

    // ── is_special_range ─────────────────────────────────────────────

    #[test]
    fn loopback_is_special() {
        assert!(is_special_range(ip4("127.0.0.1")));
        assert!(is_special_range(ip4("127.255.255.255")));
        assert!(is_special_range("::1".parse().unwrap()));
    }

    #[test]
    fn link_local_is_special() {
        assert!(is_special_range(ip4("169.254.1.1")));
        assert!(is_special_range("fe80::1".parse().unwrap()));
    }

    #[test]
    fn unique_local_v6_is_special() {
        assert!(is_special_range("fd00::1".parse().unwrap()));
        assert!(is_special_range("fc00::1".parse().unwrap()));
    }

    #[test]
    fn unspecified_is_special() {
        assert!(is_special_range(ip4("0.0.0.0")));
        assert!(is_special_range("::".parse().unwrap()));
    }

    #[test]
    fn public_ip_is_not_special() {
        assert!(!is_special_range(ip4("1.2.3.4")));
        assert!(!is_special_range(ip4("8.8.8.8")));
        assert!(!is_special_range("2606:4700::".parse().unwrap()));
    }

    #[test]
    fn ipv4_mapped_ipv6_normalized_for_specialness() {
        // ::ffff:127.0.0.1 should be treated as 127.0.0.1.
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert!(is_special_range(mapped));
    }

    // ── validate_source: master switch + empty allowlist ─────────────

    fn fixed_resolver(addrs: Vec<IpAddr>) -> ScriptedResolver {
        ScriptedResolver::new(vec![addrs])
    }

    #[tokio::test]
    async fn master_switch_off_denies_immediately() {
        let cfg = DelegationConfig::default();
        let resolver = fixed_resolver(vec![ip4("1.2.3.4")]);
        let locator = LocatorView {
            host: "example.com",
            port: 9031,
        };
        let err = validate_source(&cfg, &locator, &resolver).await.unwrap_err();
        assert_eq!(err, GateDenial::MasterSwitchOff);
    }

    #[tokio::test]
    async fn master_switch_on_with_empty_allowlist_accepts_any() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![],
        };
        let resolver = fixed_resolver(vec![ip4("1.2.3.4")]);
        let locator = LocatorView {
            host: "example.com",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("1.2.3.4"));
        assert_eq!(sa.port(), 9031);
    }

    // ── hostname / CIDR / bare-IP matching ───────────────────────────

    #[tokio::test]
    async fn hostname_entry_authorizes_public_ip() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("server-a.lan").unwrap()],
        };
        let resolver = fixed_resolver(vec![ip4("203.0.113.10")]);
        let locator = LocatorView {
            host: "Server-A.LAN.",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("203.0.113.10"));
    }

    #[tokio::test]
    async fn cidr_entry_authorizes_address_inside_range() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("10.0.0.0/8").unwrap()],
        };
        let resolver = fixed_resolver(vec![ip4("10.5.6.7")]);
        let locator = LocatorView {
            host: "host.lan",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("10.5.6.7"));
    }

    #[tokio::test]
    async fn cidr_entry_rejects_address_outside_range() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("10.0.0.0/8").unwrap()],
        };
        let resolver = fixed_resolver(vec![ip4("11.0.0.1")]);
        let locator = LocatorView {
            host: "host.lan",
            port: 9031,
        };
        let err = validate_source(&cfg, &locator, &resolver).await.unwrap_err();
        assert!(matches!(err, GateDenial::UnauthorizedAddress(_)));
    }

    #[tokio::test]
    async fn multi_a_record_with_one_outside_allowlist_denied() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("10.0.0.0/8").unwrap()],
        };
        // Two A records: one inside the CIDR, one outside.
        let resolver = fixed_resolver(vec![ip4("10.1.2.3"), ip4("11.0.0.1")]);
        let locator = LocatorView {
            host: "multi.lan",
            port: 9031,
        };
        let err = validate_source(&cfg, &locator, &resolver).await.unwrap_err();
        assert!(matches!(err, GateDenial::UnauthorizedAddress(_)));
    }

    // ── R25-F3: special-range IP-form rule ───────────────────────────

    #[tokio::test]
    async fn hostname_only_entry_does_not_authorize_loopback() {
        // evil.example.com is in the hostname allowlist but resolves to
        // 127.0.0.1. The gate must deny — this is the SSRF-via-DNS
        // scenario.
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("evil.example.com").unwrap()],
        };
        let resolver = fixed_resolver(vec![ip4("127.0.0.1")]);
        let locator = LocatorView {
            host: "evil.example.com",
            port: 9031,
        };
        let err = validate_source(&cfg, &locator, &resolver).await.unwrap_err();
        assert!(
            matches!(err, GateDenial::SpecialRangeNeedsIpAuth(_)),
            "expected SpecialRangeNeedsIpAuth, got {err:?}"
        );
    }

    #[tokio::test]
    async fn cidr_authorizing_loopback_permits_when_combined_with_hostname() {
        // Same hostname, but operator explicitly added 127.0.0.0/8 as
        // a CIDR entry. Now the loopback resolution is permitted.
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![
                parse_allow_entry("evil.example.com").unwrap(),
                parse_allow_entry("127.0.0.0/8").unwrap(),
            ],
        };
        let resolver = fixed_resolver(vec![ip4("127.0.0.1")]);
        let locator = LocatorView {
            host: "evil.example.com",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("127.0.0.1"));
    }

    #[tokio::test]
    async fn bare_ip_loopback_entry_permits_loopback_locator() {
        // Operator pinned 127.0.0.1 as a bare-IP entry; locator is the
        // literal IP.
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("127.0.0.1").unwrap()],
        };
        let resolver = ScriptedResolver::new(vec![]); // shouldn't be called
        let locator = LocatorView {
            host: "127.0.0.1",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("127.0.0.1"));
    }

    #[tokio::test]
    async fn link_local_v6_requires_ip_form() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("router.local").unwrap()],
        };
        let resolver = fixed_resolver(vec!["fe80::1".parse().unwrap()]);
        let locator = LocatorView {
            host: "router.local",
            port: 9031,
        };
        let err = validate_source(&cfg, &locator, &resolver).await.unwrap_err();
        assert!(matches!(err, GateDenial::SpecialRangeNeedsIpAuth(_)));
    }

    // ── DNS-rebinding mitigation ────────────────────────────────────

    #[tokio::test]
    async fn validates_and_returns_first_resolution_dns_rebinding_safe() {
        // The resolver script offers two different responses across
        // calls. We must only call it once, and the SocketAddr we
        // return is the address that was validated.
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("10.0.0.0/8").unwrap()],
        };
        let resolver = ScriptedResolver::new(vec![
            vec![ip4("10.1.2.3")],   // first call: in-range (validated + bound)
            vec![ip4("127.0.0.1")],  // second call: would-be rebind to loopback
        ]);
        let locator = LocatorView {
            host: "rebinder.lan",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("10.1.2.3"));
        // The second response in the script should still be there —
        // we only resolved once.
        let remaining = resolver.script.lock().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], vec![ip4("127.0.0.1")]);
    }

    // ── Public IP via hostname-only entry (R25-F3 happy path) ────────

    #[tokio::test]
    async fn public_ip_authorized_by_hostname_entry_only() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("server-a.lan").unwrap()],
        };
        let resolver = fixed_resolver(vec![ip4("1.2.3.4")]);
        let locator = LocatorView {
            host: "server-a.lan",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("1.2.3.4"));
    }

    // ── Bracketed IPv6 + IPv4-mapped IPv6 ────────────────────────────

    #[tokio::test]
    async fn bracketed_ipv6_locator_parses() {
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("::1").unwrap()],
        };
        let resolver = ScriptedResolver::new(vec![]);
        let locator = LocatorView {
            host: "[::1]",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert!(matches!(sa.ip(), IpAddr::V6(_)));
    }

    #[tokio::test]
    async fn ipv4_mapped_v6_resolution_normalized_then_matched() {
        // Resolver returns ::ffff:10.1.2.3 (IPv4-mapped IPv6); operator
        // wrote 10.0.0.0/8 in the allowlist. The mapping should flatten
        // and the match should succeed.
        let mapped: IpAddr = "::ffff:10.1.2.3".parse().unwrap();
        let cfg = DelegationConfig {
            allow_delegated_pull: true,
            allowed_source_hosts: vec![parse_allow_entry("10.0.0.0/8").unwrap()],
        };
        let resolver = fixed_resolver(vec![mapped]);
        let locator = LocatorView {
            host: "host.lan",
            port: 9031,
        };
        let sa = validate_source(&cfg, &locator, &resolver).await.unwrap();
        assert_eq!(sa.ip(), ip4("10.1.2.3"));
    }
}
