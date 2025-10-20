use eyre::{bail, eyre, Result};

/// Parsed representation of a remote `blit://` endpoint.
#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
    pub module: String,
    pub resource: Option<String>,
}

impl RemoteEndpoint {
    const SCHEME: &'static str = "blit://";
    const DEFAULT_PORT: u16 = 50051;

    /// Parses a `blit://host:port/module[/path]` style URL into host/port/module parts.
    ///
    /// The portion after the module (if any) is returned as the optional resource path.
    /// IPv6 addresses must be wrapped in `[]` (e.g., `blit://[::1]:50051/module`).
    pub fn parse(raw: &str) -> Result<Self> {
        if !raw.starts_with(Self::SCHEME) {
            bail!("remote URL must start with {} (got {})", Self::SCHEME, raw);
        }

        let body = &raw[Self::SCHEME.len()..];
        let (authority, module_and_path) = match body.split_once('/') {
            Some(parts) => parts,
            None => bail!("remote URL missing module component: {}", raw),
        };

        if module_and_path.is_empty() {
            bail!("remote URL module segment cannot be empty");
        }

        let (host, port) = parse_authority(authority)?;

        let mut segments = module_and_path.splitn(2, '/');
        let module = segments.next().unwrap();
        if module.is_empty() {
            bail!("remote URL module segment cannot be empty");
        }
        let resource = segments.next().and_then(|rest| {
            if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            }
        });

        Ok(Self {
            host,
            port,
            module: module.to_string(),
            resource,
        })
    }

    pub fn control_plane_uri(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

fn parse_authority(authority: &str) -> Result<(String, u16)> {
    if authority.is_empty() {
        bail!("remote URL missing host component");
    }

    if authority.starts_with('[') {
        // IPv6 literal
        let closing = authority
            .find(']')
            .ok_or_else(|| eyre!("unterminated IPv6 address: {}", authority))?;
        let host = &authority[1..closing];
        let port = if let Some(rem) = authority.get(closing + 1..) {
            if rem.is_empty() {
                RemoteEndpoint::DEFAULT_PORT
            } else if let Some(port) = rem.strip_prefix(':') {
                parse_port(port)?
            } else {
                bail!("invalid authority {}", authority);
            }
        } else {
            RemoteEndpoint::DEFAULT_PORT
        };
        return Ok((host.to_string(), port));
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() {
            bail!("remote URL missing host before port specification");
        }
        Ok((host.to_string(), parse_port(port)?))
    } else {
        Ok((authority.to_string(), RemoteEndpoint::DEFAULT_PORT))
    }
}

fn parse_port(raw: &str) -> Result<u16> {
    if raw.is_empty() {
        return Ok(RemoteEndpoint::DEFAULT_PORT);
    }
    raw.parse::<u16>()
        .map_err(|_| eyre!("invalid port '{}'", raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_endpoint() {
        let ep = RemoteEndpoint::parse("blit://example.com:6000/module").unwrap();
        assert_eq!(ep.host, "example.com");
        assert_eq!(ep.port, 6000);
        assert_eq!(ep.module, "module");
        assert!(ep.resource.is_none());
    }

    #[test]
    fn defaults_port() {
        let ep = RemoteEndpoint::parse("blit://example.com/module").unwrap();
        assert_eq!(ep.port, RemoteEndpoint::DEFAULT_PORT);
        assert!(ep.resource.is_none());
    }

    #[test]
    fn parses_ipv6() {
        let ep = RemoteEndpoint::parse("blit://[::1]:6000/module/sub").unwrap();
        assert_eq!(ep.host, "::1");
        assert_eq!(ep.port, 6000);
        assert_eq!(ep.module, "module");
        assert_eq!(ep.resource.as_deref(), Some("sub"));
    }

    #[test]
    fn rejects_missing_module() {
        assert!(RemoteEndpoint::parse("blit://example.com").is_err());
    }
}
