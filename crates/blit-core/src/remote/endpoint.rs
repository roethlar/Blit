use std::path::{Path, PathBuf};

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

/// Why [`RemoteEndpoint::parse`] read an input as not remote at all —
/// carried as a type, so a caller tells the cases apart by it and never by
/// the error's words (D-2026-11-24-1), which may hold the input itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRemote {
    /// The input is a local path.
    LocalPath,
    /// A remote-shaped input written with backslashes; the input as given.
    Backslashes(String),
}

impl std::fmt::Display for NotRemote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalPath => f.write_str("input appears to be a local path"),
            Self::Backslashes(got) => write!(
                f,
                "remote paths must use forward slashes, not backslashes.\n\
                 Example: server:/module/path or server://path\n\
                 Got: {got}"
            ),
        }
    }
}

impl std::error::Error for NotRemote {}

/// Parsed representation of a canonical remote endpoint.
#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
    pub path: RemotePath,
}

impl RemoteEndpoint {
    /// The one statement of blit's default daemon port (w7-6): the
    /// daemon's bind default, `blit scan`'s display elision, and the
    /// TUI's local-row fallback all reference this constant.
    pub const DEFAULT_PORT: u16 = 9031;

    pub fn parse(raw: &str) -> Result<Self> {
        // cr-uv1-1: what follows the host is a PATH, and a space at the
        // end of a path is part of its last name. The whole text was
        // trimmed here, so `host:/m/report ` named `report` — another
        // file, and `rm` removed it. Only what comes before the host is
        // set aside; a bare host, which holds no path, is trimmed below.
        let trimmed = raw.trim_start();
        if trimmed.trim_end().is_empty() {
            bail!("remote location cannot be empty");
        }

        match check_local_path(trimmed) {
            LocalPathCheck::IsLocal => return Err(NotRemote::LocalPath.into()),
            LocalPathCheck::RemoteWithBackslashes => {
                return Err(NotRemote::Backslashes(trimmed.to_string()).into());
            }
            LocalPathCheck::NotLocal => {}
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
        let (host, port) = parse_host_port(trimmed.trim_end())?;
        Ok(Self {
            host,
            port,
            path: RemotePath::Discovery,
        })
    }

    /// Where, in an operand [`parse`](Self::parse) reads as holding a
    /// path, the path below the module — or below the default root —
    /// begins: its byte offset in `raw`. What stands before it is the
    /// daemon's part of the operand, separators and all (cr-uv1-4).
    pub fn path_offset(raw: &str) -> Option<usize> {
        let lead = raw.len() - raw.trim_start().len();
        let trimmed = &raw[lead..];
        if let Some(at) = trimmed.find("://") {
            return Some(lead + at + 3);
        }
        let at = trimmed.find(":/")?;
        let module_ends = trimmed[at + 2..].find('/')?;
        Some(lead + at + 2 + module_ends + 1)
    }

    pub fn control_plane_uri(&self) -> String {
        // R58-F10: IPv6 literals must be bracketed in the URI's
        // authority component. The host field is stored bracket-
        // less (the parser strips them), so we re-bracket here.
        // A colon-containing host can only be IPv6 in our schema —
        // hostnames and IPv4 addresses never contain colons. Bare
        // `2001:db8::1:9031` is parsed by HTTP libraries as host
        // `2001` with garbage trailing, which is the bug.
        if self.host.contains(':') {
            format!("http://[{}]:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }

    /// Render just the `host[:port]` portion of this endpoint
    /// — drops the module / path suffix. Used by user-facing
    /// hints that reference the host as an argument
    /// (e.g. `blit jobs cancel <host> <id>`), where stitching
    /// the full canonical path back together would be wrong.
    /// Handles bracketed IPv6 via [`display_host`]; preserves
    /// non-default ports.
    pub fn host_port_display(&self) -> String {
        let host = display_host(&self.host);
        if self.port == Self::DEFAULT_PORT {
            host
        } else {
            format!("{}:{}", host, self.port)
        }
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

fn rel_path_to_string(path: &Path) -> String {
    // Delegate to the canonical POSIX-form helper.
    crate::path_posix::relative_path_to_posix(path)
}

fn display_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

/// Result of checking if input looks like a local path
enum LocalPathCheck {
    /// Definitely a local path
    IsLocal,
    /// Looks like a remote path with backslashes - user error
    RemoteWithBackslashes,
    /// Not a local path
    NotLocal,
}

fn check_local_path(input: &str) -> LocalPathCheck {
    if input.is_empty() {
        return LocalPathCheck::NotLocal;
    }

    let first = input.chars().next().expect("non-empty input checked above");
    if matches!(first, '.' | '/' | '\\' | '~') {
        return LocalPathCheck::IsLocal;
    }

    // UNC paths: \\server\share or //server/share
    if input.starts_with("//") || input.starts_with("\\\\") {
        return LocalPathCheck::IsLocal;
    }

    // Windows drive letter: C:\path or C:/path - check BEFORE backslash handling
    if input.len() >= 3 {
        let mut chars = input.chars();
        let drive = chars.next().expect("len >= 3 checked above");
        if drive.is_ascii_alphabetic() {
            if let Some(':') = chars.next() {
                if matches!(chars.next(), Some('\\') | Some('/')) {
                    return LocalPathCheck::IsLocal;
                }
            }
        }
    }

    // Check for backslashes - but distinguish between local paths and
    // remote paths where user mistakenly used backslashes
    if input.contains('\\') {
        // Pattern like "host:\path" or "host:\\path" suggests user meant remote
        if let Some(colon_idx) = input.find(':') {
            let before_colon = &input[..colon_idx];
            let after_colon = &input[colon_idx + 1..];
            // If before colon looks like a hostname (no slashes/backslashes)
            // and after colon starts with backslash, user probably meant remote
            // But NOT if before_colon is a single letter (Windows drive)
            if before_colon.len() > 1
                && !before_colon.contains('/')
                && !before_colon.contains('\\')
                && (after_colon.starts_with('\\') || after_colon.starts_with('/'))
            {
                return LocalPathCheck::RemoteWithBackslashes;
            }
        }
        return LocalPathCheck::IsLocal;
    }

    if input.contains('/') && !input.contains(":/") && !input.contains("://") {
        return LocalPathCheck::IsLocal;
    }

    LocalPathCheck::NotLocal
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

    /// cr-uv1-1: a path is kept as typed — a space at its end, a
    /// separator after it, a `.` as its last name. Only what stands
    /// before the host, and around a bare host, is set aside.
    #[test]
    fn a_path_is_kept_as_typed() {
        let typed = |raw: &str| match RemoteEndpoint::parse(raw).expect(raw).path {
            RemotePath::Module { module, rel_path } => {
                (module, rel_path.to_str().expect("text").to_string())
            }
            RemotePath::Root { rel_path } => {
                (String::new(), rel_path.to_str().expect("text").to_string())
            }
            RemotePath::Discovery => panic!("{raw} holds a path"),
        };
        for (raw, rel) in [
            ("srv:/m/report", "report"),
            ("srv:/m/report ", "report "),
            ("srv:/m/ report", " report"),
            ("srv:/m/dir/", "dir/"),
            ("srv:/m/dir//", "dir//"),
            ("srv:/m/work/.", "work/."),
            ("srv:/m/work/..", "work/.."),
            ("srv:/m/a/./b", "a/./b"),
            ("  srv:/m/report ", "report "),
            ("srv:/m/", ""),
            ("srv:/m/ ", " "),
        ] {
            assert_eq!(typed(raw), ("m".to_string(), rel.to_string()), "{raw:?}");
        }
        assert_eq!(typed("srv://dir/ "), (String::new(), "dir/ ".to_string()));
        // Where the path begins, for each form.
        for (raw, daemon) in [
            ("srv:/m/report", "srv:/m/"),
            ("srv:/m/", "srv:/m/"),
            ("srv://", "srv://"),
            ("srv://dir/x", "srv://"),
            ("srv:9031:/m/a/b", "srv:9031:/m/"),
            ("[::1]:9031:/m/a", "[::1]:9031:/m/"),
            ("  srv:/m/x ", "  srv:/m/"),
        ] {
            assert_eq!(
                RemoteEndpoint::path_offset(raw).map(|at| &raw[..at]),
                Some(daemon),
                "{raw:?}"
            );
        }
        assert_eq!(RemoteEndpoint::path_offset("srv"), None);
        assert_eq!(RemoteEndpoint::path_offset("srv:/m"), None);
        // A bare host holds no path: the space around it is nobody's name.
        let host = RemoteEndpoint::parse("  example.com:9130  ").expect("a host");
        assert_eq!((host.host.as_str(), host.port), ("example.com", 9130));
        assert!(RemoteEndpoint::parse("   ").is_err());
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

    #[test]
    fn errors_on_backslash_with_helpful_message() {
        let result = RemoteEndpoint::parse(r"server:\module\path");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("forward slashes"),
            "error should mention forward slashes, got: {}",
            err
        );
    }

    #[test]
    fn errors_on_double_backslash_with_helpful_message() {
        let result = RemoteEndpoint::parse(r"server:\\");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("forward slashes"),
            "error should mention forward slashes, got: {}",
            err
        );
    }

    #[test]
    fn host_port_display_drops_default_port() {
        let ep = RemoteEndpoint::parse("server:/m/path").expect("parse");
        assert_eq!(ep.host_port_display(), "server");
    }

    #[test]
    fn host_port_display_preserves_non_default_port() {
        let ep = RemoteEndpoint::parse("server:9444:/m/path").expect("parse");
        assert_eq!(ep.host_port_display(), "server:9444");
    }

    #[test]
    fn host_port_display_brackets_ipv6() {
        let ep = RemoteEndpoint::parse("[::1]:9444:/m/path").expect("parse");
        assert_eq!(ep.host_port_display(), "[::1]:9444");
    }
}
