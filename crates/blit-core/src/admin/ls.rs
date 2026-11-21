//! `ls` — a listing of a path on a daemon, or on this machine, as the
//! Unix command lists (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 5).
//!
//! The daemon lists its own disk in one request (`Ls`, which replaces
//! `List`) and says what the operand IS — a directory listed into, or one
//! line of its own — which the reply it replaces could not. This module
//! speaks gRPC and runs the same listing on a local path; a front end
//! sorts and formats.
//!
//! [`list_remote`], [`list_local`] and [`DirEntry`] are the browse layer's
//! (`crate::browse`): a flat list of rows, as it has always had.

use crate::generated::{ls_reply, LsEntry, LsKind, LsRequest};
use crate::remote::endpoint::RemoteEndpoint;
use eyre::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

pub use super::du::EntryPath;
pub use crate::listing::{Kind, ListOptions};

/// An entry as the asker holds it: its name in its directory — as text,
/// and by its exact bytes when it is not valid UTF-8 — and its facts when
/// they were asked for and could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Empty for the operand itself, which is printed as typed.
    pub name: EntryPath,
    pub facts: Option<EntryFacts>,
}

/// What the long form prints of an entry. A link's are the link's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFacts {
    pub kind: Kind,
    pub size: u64,
    pub mtime_seconds: i64,
    /// The space allocated to it, for the `total` line.
    pub allocated_bytes: u64,
    /// Where a link points, as the link holds it.
    pub link_target: Option<EntryPath>,
}

/// Something that could not be listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// `cannot access`, `cannot open directory` or `reading directory`.
    pub action: String,
    /// The entry's name in the directory; empty for the operand itself.
    pub name: EntryPath,
    /// The operating system's words: `No such file or directory`.
    pub reason: String,
}

/// What a listing said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// The operand is one line, itself, and not a directory listed into:
    /// a file, a link under the long form, anything that is no directory.
    pub itself: Option<Entry>,
    /// A directory's entries, in the order its listing gave them.
    pub entries: Vec<Entry>,
    pub failures: Vec<Failure>,
}

impl Listing {
    /// The failure that stopped the operand itself from being listed.
    pub fn of_the_operand(&self) -> Option<&Failure> {
        self.failures.iter().find(|failure| failure.name.is_start())
    }
}

fn kind_from_wire(kind: i32) -> Kind {
    match LsKind::try_from(kind) {
        Ok(LsKind::File) => Kind::File,
        Ok(LsKind::Directory) => Kind::Directory,
        Ok(LsKind::Link) => Kind::Link,
        Ok(LsKind::Fifo) => Kind::Fifo,
        Ok(LsKind::Socket) => Kind::Socket,
        Ok(LsKind::CharDevice) => Kind::CharDevice,
        Ok(LsKind::BlockDevice) => Kind::BlockDevice,
        _ => Kind::Other,
    }
}

fn entry_from_wire(entry: LsEntry) -> Entry {
    Entry {
        name: EntryPath {
            text: entry.name,
            raw: entry.raw_name,
        },
        facts: entry.facts.map(|facts| EntryFacts {
            kind: kind_from_wire(facts.kind),
            size: facts.size,
            mtime_seconds: facts.mtime_seconds,
            allocated_bytes: facts.allocated_bytes,
            link_target: facts.link_target.map(|text| EntryPath {
                text,
                raw: facts.raw_link_target,
            }),
        }),
    }
}

/// List `path` below `module` on `remote`'s daemon. The daemon answers
/// only an asker of its own contract ([`super::same_build`]). An operand
/// that cannot be listed is a [`Failure`] in the answer, not an error: the
/// asker prints it in `ls`'s words.
pub async fn list(
    remote: &RemoteEndpoint,
    module: String,
    path: EntryPath,
    options: ListOptions,
) -> Result<Listing> {
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;
    let mut stream = client
        .ls(LsRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module,
            path: path.text,
            raw_path: path.raw,
            all: options.all,
            facts: options.facts,
            link_itself: options.link_itself,
        })
        .await
        .map_err(|status| super::same_build::refusal("ls", status))?
        .into_inner();

    let mut listing = Listing::default();
    while let Some(reply) = stream
        .message()
        .await
        .map_err(|status| super::same_build::refusal("ls", status))?
    {
        match reply.reply {
            Some(ls_reply::Reply::Itself(entry)) => listing.itself = Some(entry_from_wire(entry)),
            Some(ls_reply::Reply::Directory(_)) => {}
            Some(ls_reply::Reply::Entries(batch)) => listing
                .entries
                .extend(batch.entries.into_iter().map(entry_from_wire)),
            Some(ls_reply::Reply::Failure(failure)) => listing.failures.push(Failure {
                action: failure.action,
                name: EntryPath {
                    text: failure.name,
                    raw: failure.raw_name,
                },
                reason: failure.reason,
            }),
            // A reply of a kind this build does not know: same contract,
            // so there is none.
            None => {}
        }
    }
    Ok(listing)
}

/// The same listing, of a path on this machine.
pub fn list_here(path: &Path, options: ListOptions) -> Listing {
    use crate::listing::{list, ListEvent};
    let named = |name: &std::ffi::OsStr| EntryPath::typed(&crate::find::name_bytes(name));
    let held = |entry: crate::listing::Entry| Entry {
        name: named(&entry.name),
        facts: entry.facts.map(|facts| EntryFacts {
            kind: facts.kind,
            size: facts.size,
            mtime_seconds: facts.mtime_seconds,
            allocated_bytes: facts.allocated_bytes,
            link_target: facts.link_target.map(|target| named(target.as_os_str())),
        }),
    };
    let mut listing = Listing::default();
    let listed = list(path, None, options, &mut |event| {
        match event {
            ListEvent::Itself(entry) => listing.itself = Some(held(entry)),
            ListEvent::Directory => {}
            ListEvent::Entry(entry) => listing.entries.push(held(entry)),
            ListEvent::Failure { name, error } => listing.failures.push(Failure {
                action: "cannot access".to_string(),
                name: named(&name),
                reason: crate::disk_usage::os_reason(&error),
            }),
        }
        Ok(())
    });
    if let Err(end) = listed {
        if let Some((action, error)) = end.told() {
            listing.failures.push(Failure {
                action: action.to_string(),
                name: EntryPath::default(),
                reason: crate::disk_usage::os_reason(error),
            });
        }
    }
    listing
}

/// One row of a directory listing, for the browse layer. Same shape for
/// local and remote modes.
#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_seconds: i64,
}

/// Outcome of `list_local`. Discriminates "target is a directory,
/// here are its contents" from "target is a single non-directory
/// (file, device, FIFO, socket, symlink); here is its stat
/// summary."
#[derive(Debug, Clone)]
pub enum LocalListing {
    /// Target was a directory. Entries are sorted by path.
    Directory { entries: Vec<DirEntry> },
    /// Target was a single non-directory. Single-entry summary,
    /// basename in `entry.name`.
    Target { entry: DirEntry },
}

impl LocalListing {
    /// Flatten into a single entry vec.
    pub fn into_entries(self) -> Vec<DirEntry> {
        match self {
            LocalListing::Directory { entries } => entries,
            LocalListing::Target { entry } => vec![entry],
        }
    }
}

/// The rows under a remote `module:path`, for the browse layer: every
/// entry of a directory, names beginning with `.` included, sorted by
/// name; or the one row of an operand that is no directory, under its own
/// name. An operand that cannot be listed is an error.
pub async fn list_remote(
    remote: &RemoteEndpoint,
    module: String,
    path: String,
) -> Result<Vec<DirEntry>> {
    let options = ListOptions {
        all: true,
        facts: true,
        link_itself: false,
    };
    let listing = list(remote, module, EntryPath::typed(path.as_bytes()), options).await?;
    if let Some(failure) = listing.of_the_operand() {
        bail!("{} '{path}': {}", failure.action, failure.reason);
    }
    let row = |name: String, entry: &Entry| DirEntry {
        name,
        is_dir: entry
            .facts
            .as_ref()
            .is_some_and(|facts| facts.kind == Kind::Directory),
        size: entry.facts.as_ref().map_or(0, |facts| facts.size),
        mtime_seconds: entry.facts.as_ref().map_or(0, |facts| facts.mtime_seconds),
    };
    if let Some(itself) = &listing.itself {
        let name = String::from_utf8_lossy(super::operand::final_name(path.as_bytes()));
        return Ok(vec![row(name.into_owned(), itself)]);
    }
    let mut rows: Vec<DirEntry> = listing
        .entries
        .iter()
        .filter(|entry| !matches!(entry.name.bytes(), b"." | b".."))
        .map(|entry| row(entry.name.text.clone(), entry))
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

/// Stat-or-list a local path, for the browse layer: a directory expands
/// into its entries (sorted by path); any non-directory (regular file,
/// device, FIFO, socket, symlink to a non-directory) returns a
/// single-entry `Target`. Recursion is the caller's responsibility.
pub fn list_local(path: &Path) -> Result<LocalListing> {
    let metadata =
        fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;

    if metadata.is_dir() {
        let mut entries: Vec<DirEntry> = Vec::new();
        let mut paths: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("collecting entries for {}", path.display()))?;
        paths.sort_by_key(|e| e.path());
        for entry in paths {
            let meta = entry
                .metadata()
                .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
            let is_dir = meta.is_dir();
            let size = if is_dir { 0 } else { meta.len() };
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir,
                size,
                mtime_seconds: crate::wire_metadata::mtime_seconds(&meta).unwrap_or(0),
            });
        }
        Ok(LocalListing::Directory { entries })
    } else {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string());
        Ok(LocalListing::Target {
            entry: DirEntry {
                name,
                is_dir: false,
                size: metadata.len(),
                mtime_seconds: crate::wire_metadata::mtime_seconds(&metadata).unwrap_or(0),
            },
        })
    }
}
