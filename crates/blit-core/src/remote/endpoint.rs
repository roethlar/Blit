use std::path::PathBuf;

use eyre::{bail, eyre, Result};

/// Canonical remote path variants.
#[derive(Debug, Clone)]
pub enum RemotePath {
    /// Addressed via `server:/module/...`
    Module { module: String, rel_path: PathBuf },
    /// Addressed via `server://...` (default root/export)
    Root { rel_path: PathBuf },
    /// Discovery form (`server` or `server:port`)
    Discovery,
}

/// Parsed representation of a canonical remote endpoint.
#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
    pub path: RemotePath,
}

impl RemoteEndpoint {
    const DEFAULT_PORT: u16 = 9031;

    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("remote location cannot be empty");
        }

        if looks_like_local_path(trimmed) {
            bail!("input appears to be a local path");
        }

        if let Some(idx) = trimmed.find("://") {
            // Root export (server://path)
            let host_port = &trimmed[..idx];
            let remainder = &trimmed[idx + 3..];
            let (host, port) = parse_host_port(host_port)?;
            let rel = normalize_relative_path_buf(remainder);
            return Ok(Self {
                host,
                port,
                path: RemotePath::Root { rel_path: rel },
            });
        }

        if let Some(idx) = trimmed.find(":/") {
            // Module export (server:/module/...)
            let host_port = &trimmed[..idx];
            let remainder = &trimmed[idx + 2..];
            let (host, port) = parse_host_port(host_port)?;

            let slash_idx = remainder.find('/').ok_or_else(|| {
                eyre!(
                    "module path must end with '/' (e.g., server:/module/ or server:/module/path)"
                )
            })?;

            let module = &remainder[..slash_idx];
            if module.is_empty() {
                bail!("module name cannot be empty; expected server:/module/...");
            }
            let rest = &remainder[slash_idx + 1..];
            let rel = normalize_relative_path_buf(rest);

            return Ok(Self {
                host,
                port,
                path: RemotePath::Module {
                    module: module.to_string(),
                    rel_path: rel,
                },
            });
        }

        // Discovery (server or server:port)
        let (host, port) = parse_host_port(trimmed)?;
        Ok(Self {
            host,
            port,
            path: RemotePath::Discovery,
        })
    }

    pub fn control_plane_uri(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Display helper used by CLI for canonical formatting.
    pub fn display(&self) -> String {
        let host = display_host(&self.host);
        let host_port = if self.port == Self::DEFAULT_PORT {
            host
        } else {
            format!("{}:{}", host, self.port)
        };

        match &self.path {
            RemotePath::Module { module, rel_path } => {
                let mut path = format!("{}:/{}", host_port, module);
                if rel_path.as_os_str().is_empty() {
                    path.push('/');
                } else {
                    path.push('/');
                    path.push_str(&rel_path_to_string(rel_path));
                }
                path
            }
            RemotePath::Root { rel_path } => {
                if rel_path.as_os_str().is_empty() {
                    format!("{host_port}://")
                } else {
                    format!("{}://{}", host_port, rel_path_to_string(rel_path))
                }
            }
            RemotePath::Discovery => host_port,
        }
    }
}

fn parse_host_port(authority: &str) -> Result<(String, u16)> {
    if authority.is_empty() {
        bail!("remote location missing host");
    }

    if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal
        let closing = stripped
            .find(']')
            .ok_or_else(|| eyre!("unterminated IPv6 address: {}", authority))?;
        let host = &stripped[..closing];
        let remainder = &stripped[closing + 1..];
        let port = if let Some(port_str) = remainder.strip_prefix(':') {
            parse_port(port_str)?
        } else if remainder.is_empty() {
            RemoteEndpoint::DEFAULT_PORT
        } else {
            bail!("invalid host specification: {}", authority);
        };
        return Ok((host.to_string(), port));
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() {
            bail!("remote location missing host before ':'");
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

fn normalize_relative_path_buf(raw: &str) -> PathBuf {
    if raw.is_empty() {
        PathBuf::new()
    } else {
        let trimmed = raw.trim_start_matches('/');
        if trimmed.is_empty() {
            PathBuf::new()
        } else {
            PathBuf::from(trimmed)
        }
    }
}

fn rel_path_to_string(path: &PathBuf) -> String {
    path.iter()
        .map(|component| component.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn display_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

fn looks_like_local_path(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }

    let first = input.chars().next().unwrap();
    if matches!(first, '.' | '/' | '\\' | '~') {
        return true;
    }

    if input.starts_with("//") || input.starts_with("\\\\") {
        return true;
    }

    if input.contains('\\') {
        return true;
    }

    if input.contains('/') && !input.contains(":/") && !input.contains("://") {
        return true;
    }

    if input.len() >= 3 {
        let mut chars = input.chars();
        let drive = chars.next().unwrap();
        if drive.is_ascii_alphabetic() {
            if let Some(':') = chars.next() {
                if matches!(chars.next(), Some('\\') | Some('/')) {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module_root() {
        let ep = RemoteEndpoint::parse("example.com:/media/").unwrap();
        assert_eq!(ep.host, "example.com");
        assert_eq!(ep.port, RemoteEndpoint::DEFAULT_PORT);
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "media");
                assert!(rel_path.as_os_str().is_empty());
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn parses_module_with_subpath() {
        let ep = RemoteEndpoint::parse("example.com:9000:/data/projects/foo").unwrap();
        assert_eq!(ep.port, 9000);
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "data");
                assert_eq!(rel_path_to_string(rel_path), "projects/foo");
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn parses_root_path() {
        let ep = RemoteEndpoint::parse("example.com://backups").unwrap();
        match ep.path {
            RemotePath::Root { ref rel_path } => {
                assert_eq!(rel_path_to_string(rel_path), "backups");
            }
            _ => panic!("expected root path"),
        }
    }

    #[test]
    fn parses_discovery_host_only() {
        let ep = RemoteEndpoint::parse("example.com").unwrap();
        matches!(ep.path, RemotePath::Discovery);
    }

    #[test]
    fn parses_discovery_with_port() {
        let ep = RemoteEndpoint::parse("example.com:9130").unwrap();
        assert_eq!(ep.port, 9130);
        matches!(ep.path, RemotePath::Discovery);
    }

    #[test]
    fn parses_ipv6_module() {
        let ep = RemoteEndpoint::parse("[2001:db8::1]:/share/").unwrap();
        assert_eq!(ep.host, "2001:db8::1");
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "share");
                assert!(rel_path.as_os_str().is_empty());
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn errors_on_missing_module_slash() {
        assert!(RemoteEndpoint::parse("example.com:/module").is_err());
    }
}
