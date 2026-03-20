//! Ordered include/exclude filter rules with first-match-wins semantics.
//!
//! Supports glob patterns, size constraints, age constraints, explicit file
//! lists (`--files-from`), and rule files (`--filter-from`).

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use eyre::{Context, Result};
use globset::{Glob, GlobMatcher};

#[derive(Clone, Debug)]
enum RuleKind {
    Include,
    Exclude,
}

#[derive(Clone, Debug)]
struct Rule {
    kind: RuleKind,
    matcher: GlobMatcher,
}

/// Ordered filter rules with first-match-wins semantics.
///
/// When evaluating a path, rules are checked in order. The first matching
/// rule determines whether the path is included or excluded. If no rule
/// matches, the path is included by default.
///
/// Size and age constraints are checked after rule evaluation and can
/// reject files that rules would otherwise include (unless an explicit
/// include rule matched).
#[derive(Clone, Debug)]
pub struct FilterRules {
    rules: Vec<Rule>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    min_age: Option<Duration>,
    max_age: Option<Duration>,
    files_from: Option<HashSet<PathBuf>>,
    reference_time: SystemTime,
}

impl Default for FilterRules {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            files_from: None,
            reference_time: SystemTime::now(),
        }
    }
}

impl FilterRules {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if no rules or constraints are configured.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
            && self.min_size.is_none()
            && self.max_size.is_none()
            && self.min_age.is_none()
            && self.max_age.is_none()
            && self.files_from.is_none()
    }

    /// Add an exclude pattern (glob syntax).
    pub fn exclude(mut self, pattern: &str) -> Result<Self> {
        let glob = Glob::new(pattern)
            .with_context(|| format!("invalid exclude pattern: {pattern}"))?;
        self.rules.push(Rule {
            kind: RuleKind::Exclude,
            matcher: glob.compile_matcher(),
        });
        Ok(self)
    }

    /// Add an include pattern (glob syntax).
    pub fn include(mut self, pattern: &str) -> Result<Self> {
        let glob = Glob::new(pattern)
            .with_context(|| format!("invalid include pattern: {pattern}"))?;
        self.rules.push(Rule {
            kind: RuleKind::Include,
            matcher: glob.compile_matcher(),
        });
        Ok(self)
    }

    pub fn min_size(mut self, bytes: u64) -> Self {
        self.min_size = Some(bytes);
        self
    }

    pub fn max_size(mut self, bytes: u64) -> Self {
        self.max_size = Some(bytes);
        self
    }

    pub fn min_age(mut self, duration: Duration) -> Self {
        self.min_age = Some(duration);
        self
    }

    pub fn max_age(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    /// Load ordered rules from a file. Lines starting with `+ ` are include
    /// rules, lines starting with `- ` are exclude rules. Bare lines without
    /// a prefix are treated as exclude rules. Empty lines and `#` comments
    /// are skipped.
    pub fn load_rules_from(mut self, path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening filter rules file {}", path.display()))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(pattern) = trimmed.strip_prefix("+ ") {
                self = self.include(pattern)?;
            } else if let Some(pattern) = trimmed.strip_prefix("- ") {
                self = self.exclude(pattern)?;
            } else {
                self = self.exclude(trimmed)?;
            }
        }
        Ok(self)
    }

    /// Load explicit file list. Only listed paths are transferred.
    pub fn files_from(mut self, path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening files-from list {}", path.display()))?;
        let mut set = HashSet::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                set.insert(PathBuf::from(trimmed));
            }
        }
        self.files_from = Some(set);
        Ok(self)
    }

    /// Check whether a file should be included.
    ///
    /// `rel_path` is relative to the transfer root.
    pub fn allows_file(&self, rel_path: &Path, size: u64, mtime: Option<SystemTime>) -> bool {
        if let Some(ref allowed) = self.files_from {
            return allowed.contains(rel_path);
        }

        let explicit_include = matches!(self.eval_rules(rel_path), RuleMatch::Include);

        if matches!(self.eval_rules(rel_path), RuleMatch::Exclude) {
            return false;
        }

        // Size and age filters only apply when there was no explicit include
        if !explicit_include {
            if let Some(min) = self.min_size {
                if size < min {
                    return false;
                }
            }
            if let Some(max) = self.max_size {
                if size > max {
                    return false;
                }
            }

            if let Some(mtime) = mtime {
                if let Ok(age) = self.reference_time.duration_since(mtime) {
                    if let Some(min_age) = self.min_age {
                        if age < min_age {
                            return false;
                        }
                    }
                    if let Some(max_age) = self.max_age {
                        if age > max_age {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Check whether a directory should be traversed.
    pub fn allows_dir(&self, rel_path: &Path) -> bool {
        if self.files_from.is_some() {
            return true;
        }
        !matches!(self.eval_rules(rel_path), RuleMatch::Exclude)
    }

    fn eval_rules(&self, rel_path: &Path) -> RuleMatch {
        let path_str = rel_path.to_string_lossy();
        let filename = rel_path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();

        for rule in &self.rules {
            if rule.matcher.is_match(path_str.as_ref())
                || rule.matcher.is_match(filename.as_ref())
            {
                return match rule.kind {
                    RuleKind::Include => RuleMatch::Include,
                    RuleKind::Exclude => RuleMatch::Exclude,
                };
            }
        }

        RuleMatch::NoMatch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleMatch {
    Include,
    Exclude,
    NoMatch,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a human-readable size like "100K", "10M", "1G", "1.5Mi" into bytes.
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(eyre::eyre!("empty size string"));
    }

    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("Ti") {
        (n, 1u64 << 40)
    } else if let Some(n) = s.strip_suffix("Gi") {
        (n, 1u64 << 30)
    } else if let Some(n) = s.strip_suffix("Mi") {
        (n, 1u64 << 20)
    } else if let Some(n) = s.strip_suffix("Ki") {
        (n, 1u64 << 10)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1_000_000_000_000u64)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1_000_000_000)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1_000_000)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1_000)
    } else {
        (s, 1)
    };

    let num: f64 = num_str
        .parse()
        .map_err(|_| eyre::eyre!("invalid size number: {num_str}"))?;
    Ok((num * multiplier as f64) as u64)
}

/// Parse a human-readable duration like "30s", "5m", "1h", "7d", "1h30m".
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(eyre::eyre!("empty duration string"));
    }

    let mut total_secs = 0u64;
    let mut num_buf = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else {
            if num_buf.is_empty() {
                return Err(eyre::eyre!("invalid duration: {s}"));
            }
            let num: f64 = num_buf
                .parse()
                .map_err(|_| eyre::eyre!("invalid number in duration: {s}"))?;
            num_buf.clear();

            let multiplier = match ch {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                'w' => 604800,
                _ => return Err(eyre::eyre!("unknown duration unit '{ch}' in: {s}")),
            };
            total_secs += (num * multiplier as f64) as u64;
        }
    }

    // Bare number without unit = seconds
    if !num_buf.is_empty() {
        let num: f64 = num_buf
            .parse()
            .map_err(|_| eyre::eyre!("invalid number in duration: {s}"))?;
        total_secs += num as u64;
    }

    if total_secs == 0 {
        return Err(eyre::eyre!("duration must be non-zero: {s}"));
    }

    Ok(Duration::from_secs(total_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_allows_everything() {
        let f = FilterRules::new();
        assert!(f.allows_file(Path::new("foo.txt"), 100, None));
        assert!(f.allows_dir(Path::new("bar")));
        assert!(f.is_empty());
    }

    #[test]
    fn exclude_pattern_blocks_match() {
        let f = FilterRules::new().exclude("*.tmp").unwrap();
        assert!(!f.allows_file(Path::new("test.tmp"), 100, None));
        assert!(f.allows_file(Path::new("test.txt"), 100, None));
    }

    #[test]
    fn include_before_exclude() {
        let f = FilterRules::new()
            .include("important.log")
            .unwrap()
            .exclude("*.log")
            .unwrap();
        assert!(f.allows_file(Path::new("important.log"), 100, None));
        assert!(!f.allows_file(Path::new("debug.log"), 100, None));
    }

    #[test]
    fn first_match_wins() {
        let f = FilterRules::new()
            .exclude("*.log")
            .unwrap()
            .include("*.log")
            .unwrap();
        assert!(!f.allows_file(Path::new("test.log"), 100, None));
    }

    #[test]
    fn size_filters() {
        let f = FilterRules::new().min_size(100).max_size(1000);
        assert!(!f.allows_file(Path::new("small.txt"), 50, None));
        assert!(f.allows_file(Path::new("right.txt"), 500, None));
        assert!(!f.allows_file(Path::new("big.txt"), 2000, None));
    }

    #[test]
    fn age_filters() {
        let now = SystemTime::now();
        let old = now - Duration::from_secs(3600);
        let recent = now - Duration::from_secs(60);

        let f = FilterRules {
            min_age: Some(Duration::from_secs(600)),
            reference_time: now,
            ..Default::default()
        };
        assert!(f.allows_file(Path::new("old.txt"), 100, Some(old)));
        assert!(!f.allows_file(Path::new("new.txt"), 100, Some(recent)));
    }

    #[test]
    fn dir_exclude() {
        let f = FilterRules::new().exclude("node_modules").unwrap();
        assert!(!f.allows_dir(Path::new("node_modules")));
        assert!(f.allows_dir(Path::new("src")));
    }

    #[test]
    fn path_glob_matching() {
        let f = FilterRules::new().exclude("logs/**").unwrap();
        assert!(!f.allows_file(Path::new("logs/app.log"), 100, None));
        assert!(f.allows_file(Path::new("src/app.rs"), 100, None));
    }

    #[test]
    fn parse_size_units() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("10K").unwrap(), 10_000);
        assert_eq!(parse_size("10M").unwrap(), 10_000_000);
        assert_eq!(parse_size("1G").unwrap(), 1_000_000_000);
        assert_eq!(parse_size("1Mi").unwrap(), 1 << 20);
        assert_eq!(parse_size("1Gi").unwrap(), 1 << 30);
        assert_eq!(parse_size("1.5M").unwrap(), 1_500_000);
    }

    #[test]
    fn parse_duration_units() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("7d").unwrap(), Duration::from_secs(604800));
        assert_eq!(
            parse_duration("1h30m").unwrap(),
            Duration::from_secs(5400)
        );
        assert_eq!(parse_duration("90").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn parse_errors() {
        assert!(parse_size("").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("0s").is_err());
    }

    #[test]
    fn files_from_list() {
        let dir = tempfile::tempdir().unwrap();
        let list_file = dir.path().join("files.txt");
        std::fs::write(&list_file, "src/main.rs\nREADME.md\n").unwrap();

        let f = FilterRules::new().files_from(&list_file).unwrap();
        assert!(f.allows_file(Path::new("src/main.rs"), 100, None));
        assert!(f.allows_file(Path::new("README.md"), 100, None));
        assert!(!f.allows_file(Path::new("Cargo.toml"), 100, None));
        // files-from mode traverses all dirs
        assert!(f.allows_dir(Path::new("anything")));
    }

    #[test]
    fn filter_rules_file() {
        let dir = tempfile::tempdir().unwrap();
        let rules_file = dir.path().join("rules.txt");
        std::fs::write(
            &rules_file,
            "# keep important logs\n+ important.log\n- *.log\n- *.tmp\n",
        )
        .unwrap();

        let f = FilterRules::new().load_rules_from(&rules_file).unwrap();
        assert!(f.allows_file(Path::new("important.log"), 100, None));
        assert!(!f.allows_file(Path::new("debug.log"), 100, None));
        assert!(!f.allows_file(Path::new("scratch.tmp"), 100, None));
        assert!(f.allows_file(Path::new("main.rs"), 100, None));
    }
}
