use super::{DiskUsageSender, FindSender, RmSender};
use blit_core::disk_usage::{disk_usage, os_reason, Stopped, UsageEnd, UsageEvent, UsageOptions};
use blit_core::find::{find, FindEnd, FindEvent, FindOptions};
use blit_core::generated::{
    du_reply, find_paths_reply, rm_reply, DfReply, DuFailure, DuReply, DuRow, FindBatch,
    FindFailure, FindPath, FindPathsReply, RmDone, RmFailure, RmFailures, RmReply,
};
use std::fs;
use std::path::{Path, PathBuf};
use tonic::Status;

use super::util::{internal_err, io_to_status, resolve_relative_path, response_channel_closed};
use blit_core::path_posix::request_path_to_posix;

/// One operand of a removal as the request names it: where it is and how
/// it was typed, or why it cannot be removed at all.
pub(crate) type RmOperand = Result<TypedOperand, String>;

/// An operand's place below the module, and what its writing said
/// (cr-uv1-1, [`blit_core::admin::operand`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypedOperand {
    pub rel: PathBuf,
    /// Typed with a separator after it: a directory, or nothing.
    pub as_directory: bool,
}

/// Why the module's own root is not removed.
const THE_MODULE_ROOT: &str = "refusing to remove the module's root; name a path inside it";

/// A move's removal of a path no ticket it presents is for (cr-uv4-1).
const NOT_THE_PATH_THAT_WAS_SENT: &str =
    "the account of what was sent is of another path: nothing here was sent by that transfer";

/// `rm`'s own words for an operand whose last name is `.` or `..`.
const DOT_OR_DOT_DOT: &str = "refusing to remove '.' or '..' directory";

/// Where a read starts — `du`, `df`, `find` — from the path as typed: its
/// place below the module, and whether it was typed as a directory.
#[allow(clippy::result_large_err)]
pub(crate) fn read_start(typed: &[u8]) -> Result<TypedOperand, Status> {
    use blit_core::admin::operand;
    let rel = if operand::is_the_root(typed) {
        PathBuf::from(".")
    } else {
        resolve_typed(typed)?
    };
    Ok(TypedOperand {
        rel,
        as_directory: operand::names_a_directory(typed),
    })
}

/// A path as a request carries it (cr-uv1-4): its exact bytes when the
/// asker sent them — the path is then not valid UTF-8 — and its text
/// otherwise.
pub(crate) fn typed_bytes<'a>(text: &'a str, raw: Option<&'a [u8]>) -> &'a [u8] {
    match raw {
        Some(raw) if !raw.is_empty() => raw,
        _ => text.as_bytes(),
    }
}

/// A typed path's place below the module: its components are what stands
/// between `/`, each one name, whatever it holds (cr-uv3-3,
/// `path_safety::typed_components`) — relative, no `..`. A system that
/// cannot hold a component as one name refuses it by saying so. The
/// module itself is `.`.
#[allow(clippy::result_large_err)]
fn resolve_typed(typed: &[u8]) -> Result<PathBuf, Status> {
    let path = blit_core::path_safety::typed_components(typed, cfg!(windows))
        .map_err(|error| Status::invalid_argument(format!("path not allowed: {error}")))?;
    Ok(if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    })
}

impl TypedOperand {
    /// The operand below `root`, written for the system as it was typed:
    /// with a separator after it when it was typed as a directory. The
    /// system then resolves a link standing there and refuses what is no
    /// directory, as it does for the command run on this disk.
    ///
    /// The module itself is `root` as it stands (cr-uv5-5), with nothing
    /// after it: it was `root/.`, which the system refuses for a module
    /// that is one file — `ls`, `du` and `find` of such a module ended
    /// in a refusal for a path out of the module.
    pub(crate) fn below(&self, root: &Path) -> PathBuf {
        let mut path = if self.rel == Path::new(".") {
            root.to_path_buf()
        } else {
            root.join(&self.rel)
        };
        if self.as_directory {
            path.push("");
        }
        path
    }
}

/// F2, for where a read starts: it lies in the module, or the request is
/// refused before anything is read.
///
/// cr-uv5-5: a start the system will not LOOK UP — a directory on its way
/// cannot be searched, a link there leads round in a ring, a name is too
/// long — is no path out of the module, and was refused as one: `ls` of a
/// file below a directory without search permission ended the request,
/// exit 1 and nothing in `--json`, where `ls` says `cannot access …:
/// Permission denied` and exits 2. What is judged is the deepest part of
/// the start the system does look up — as a start that is not there has
/// always been judged by its deepest ancestor that is. That part in the
/// module, the read goes on to the start, fails there as the system
/// fails, and says so in its command's own words; nothing is read past
/// what could not be looked up. That part out of the module, the request
/// is refused as before.
#[allow(clippy::result_large_err)]
fn start_is_contained(module_root: &Path, start_abs: PathBuf) -> Result<Readable, Status> {
    use std::io::ErrorKind::{NotADirectory, NotFound};
    // cr-uv7-2: nothing at all at the root's path — the module's folder
    // was moved or removed — is no way out of the module, for the module
    // itself or anything below it; it is told in its command's own words
    // (`cannot access …: No such file or directory`, and its exit status),
    // as for any path that is not there: the request was refused whole,
    // and `--json` was empty. cr-uv8-2: told WITH THE ERROR SEEN HERE, and
    // nothing looked up again — the read that went on to find nothing
    // could find, by then, a link to somewhere else at the root's path.
    // Something there that cannot be resolved — a link to nothing, a ring
    // — is judged below, and refused, as before.
    match fs::symlink_metadata(module_root) {
        Err(error) if error.kind() == NotFound => return Ok(Readable::Gone(error)),
        _ => {}
    }
    // The module itself, however it was typed, is in the module. Judged
    // by its ancestors — as what is no directory, typed as one, is — a
    // module that is one file would be out of itself.
    //
    // cr-uv6-2: but it is the module only while its path still resolves
    // to ITSELF. The root is a path with no link in it, resolved when the
    // daemon started; a link put in its place since leads somewhere
    // else, and a listing of "the module" was a listing of that — passed
    // here on the strength of the two paths being spelled alike. Asked
    // of the root as the daemon holds it, with nothing after it: a
    // module that is one file, typed as a directory, is still the module,
    // and is told `Not a directory` by the read.
    if start_abs.components().eq(module_root.components()) {
        return match fs::canonicalize(module_root) {
            Ok(now) if now == module_root => Ok(Readable::At(start_abs)),
            Ok(now) => Err(Status::permission_denied(format!(
                "path containment: the module's root '{}' now resolves to '{}'",
                module_root.display(),
                now.display()
            ))),
            Err(error) => Err(Status::permission_denied(format!(
                "path containment: the module's root '{}' cannot be resolved: {error}",
                module_root.display()
            ))),
        };
    }
    let mut looked_up = start_abs.to_path_buf();
    while let Err(error) = fs::canonicalize(&looked_up) {
        // Not there: `verify_contained` judges that by its ancestors.
        if matches!(error.kind(), NotFound | NotADirectory) || !looked_up.pop() {
            break;
        }
    }
    blit_core::path_safety::verify_contained(module_root, &looked_up)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;
    Ok(Readable::At(start_abs))
}

/// What [`start_is_contained`] makes of a read's start (cr-uv8-2): the
/// path to read — the only way a read gets one — or what to tell instead.
#[derive(Debug)]
enum Readable {
    At(PathBuf),
    /// Nothing at the module root's path: this error, told as the
    /// command tells a start that is not there.
    Gone(std::io::Error),
}

/// Read a removal's operands. One that cannot be an operand — the
/// module's root, a path with `..` in it — is THAT operand's failure and
/// no more (cr-uv1-3): it used to refuse the request whole, where `rm`
/// goes on to the operands it can remove.
///
/// Each is read as typed (cr-uv1-1). One whose last name is `.` or `..`
/// is refused in `rm`'s words — `work/.` was read as `work`, and `work`
/// removed with everything in it; one typed with a separator after it is
/// a directory or nothing.
///
/// `raw` holds, at an operand's place, its exact bytes when it is not
/// valid UTF-8 (cr-uv1-4); it is empty, or as long as `paths`.
#[allow(clippy::result_large_err)]
pub(crate) fn read_operands(
    paths: Vec<String>,
    raw: Vec<Vec<u8>>,
) -> Result<Vec<RmOperand>, Status> {
    use blit_core::admin::operand;
    if !raw.is_empty() && raw.len() != paths.len() {
        return Err(Status::invalid_argument(format!(
            "{} paths and {} of their byte forms: one for each, or none",
            paths.len(),
            raw.len()
        )));
    }
    Ok(paths
        .iter()
        .enumerate()
        .map(|(at, text)| {
            let typed = typed_bytes(text, raw.get(at).map(Vec::as_slice));
            if operand::is_the_root(typed) {
                return Err(THE_MODULE_ROOT.to_string());
            }
            if operand::names_dot(typed) {
                return Err(DOT_OR_DOT_DOT.to_string());
            }
            let rel = resolve_typed(typed).map_err(|status| status.message().to_string())?;
            Ok(TypedOperand {
                rel,
                as_directory: operand::names_a_directory(typed),
            })
        })
        .collect())
}

/// `rm`'s operands, each removed as the command removes it
/// (UTILITY_VERBS_LOCAL_PARITY slice 2): one after another, an operand
/// that fails not stopping the next, and every entry that could not be
/// removed reported with the operand it belongs to.
///
/// The answer is sent as it is made (cr-uv1-5): each failure as the
/// removal meets it, in messages bounded by their size, then `RmDone`
/// with the count. It used to be one reply, holding at most 1,000
/// failures for an operand and a flag for the rest, and built whole
/// before any of it was sent — a reply that could outgrow what a message
/// may hold only after the removal had happened.
pub(crate) fn stream_removal(
    module_path: PathBuf,
    canonical_root: PathBuf,
    operands: Vec<RmOperand>,
    options: blit_core::deletion::RmOptions,
    sent: Option<&[std::sync::Arc<blit_core::moved::Closed>]>,
    sender: &RmSender,
) -> Result<(), Status> {
    let closed = || response_channel_closed("sending removal result");
    let mut batch = FailureBatch {
        failures: Vec::new(),
        bytes: 0,
        sent: std::time::Instant::now(),
        sender,
    };
    let removed = remove_each(
        &module_path,
        &canonical_root,
        &operands,
        options,
        sent,
        // cr-uv2-3: nobody is reading the answer any more.
        &|| sender.is_closed(),
        &mut |failure| batch.add(failure),
    )?;
    batch.send().map_err(|Stopped| closed())?;
    sender
        .blocking_send(Ok(RmReply {
            reply: Some(rm_reply::Reply::Done(RmDone {
                entries_removed: removed,
            })),
        }))
        .map_err(|_| closed())
}

/// Each operand removed in turn; `failed` is told of every entry that
/// could not be, with its operand's place, as it is met. Returns the count
/// of entries removed.
///
/// `called_off` says the asker has gone (cr-uv2-3). The removal asks it
/// before each operand is touched and before each entry below one: a
/// removal that met no failure had nothing to send, so it learned that
/// nobody was listening only when it came to say it was done — with every
/// operand removed. An asker that goes away is one that stopped the
/// command.
fn remove_each(
    module_path: &Path,
    canonical_root: &Path,
    operands: &[RmOperand],
    options: blit_core::deletion::RmOptions,
    sent: Option<&[std::sync::Arc<blit_core::moved::Closed>]>,
    called_off: &dyn Fn() -> bool,
    failed: &mut dyn FnMut(RmFailure) -> Result<(), Stopped>,
) -> Result<u64, Status> {
    let closed = || response_channel_closed("sending removal result");
    let mut entries_removed = 0u64;
    for (operand, rel) in operands.iter().enumerate() {
        let rel = match rel {
            Ok(rel) => rel,
            Err(reason) => {
                failed(RmFailure {
                    operand: operand as u32,
                    relative_path: String::new(),
                    raw_relative_path: None,
                    reason: reason.clone(),
                })
                .map_err(|Stopped| closed())?;
                continue;
            }
        };
        let place = module_path.join(&rel.rel);
        // cr-uv4-1: an account is of the path its session read, and
        // answers for that path alone. Tickets presented for another
        // path say nothing of what was sent from this one.
        let here: Option<Vec<&blit_core::moved::Closed>> = sent.map(|accounts| {
            accounts
                .iter()
                .map(std::sync::Arc::as_ref)
                .filter(|account| account.is_of(&place))
                .collect()
        });
        if here.as_ref().is_some_and(Vec::is_empty) {
            failed(RmFailure {
                operand: operand as u32,
                relative_path: String::new(),
                raw_relative_path: None,
                reason: NOT_THE_PATH_THAT_WAS_SENT.to_string(),
            })
            .map_err(|Stopped| closed())?;
            continue;
        }
        // cr-uv3-1: a move's removal takes what its sessions sent. An
        // entry that is no directory goes only if it is a regular file
        // one of the accounts holds — this path, as the file it was when
        // the account was written (cr-uv4-1: its size, its time to the
        // nanosecond, when it was last changed, which file it is);
        // anything else stays and is told, in words that say which it is.
        let unsent = |relative_path: &Path, met: &blit_core::deletion::Met| {
            use blit_core::deletion::MetKind;
            let accounts = here.as_ref()?;
            let why = match met.kind {
                MetKind::Link => "it is a symbolic link, which a move does not carry",
                MetKind::Other => "it is not a regular file, and a move carries only those",
                // cr-uv7-1: and where the version cannot answer for what
                // the file holds — a later name of one with several — the
                // account must hold what it holds.
                MetKind::File
                    if accounts.iter().any(|account| {
                        let path = sent_path(relative_path);
                        account.holds(&path, &met.version)
                            && match &met.content {
                                blit_core::deletion::Content::NotAsked => true,
                                blit_core::deletion::Content::Digest(digest) => {
                                    account.holds_content(&path, digest)
                                }
                                blit_core::deletion::Content::Unreadable => false,
                            }
                    }) =>
                {
                    return None;
                }
                MetKind::File => {
                    "it is not a file the move sent: it is new, or has changed since it was read"
                }
            };
            Some(std::io::Error::other(why))
        };
        let keep: Option<blit_core::deletion::Keep<'_>> = here.is_some().then_some(&unsent);
        let removed = blit_core::deletion::remove_what_may_go(
            &place,
            rel.as_directory,
            Some(canonical_root),
            options,
            Some(called_off),
            keep,
            &mut |failure| {
                failed(RmFailure {
                    operand: operand as u32,
                    relative_path: wire_path(&failure.relative_path),
                    raw_relative_path: blit_core::raw_name::raw_relative_bytes(
                        &failure.relative_path,
                    ),
                    reason: os_reason(&failure.error),
                })
            },
        );
        match removed {
            Ok(removed) => entries_removed += removed,
            // The asker has gone: nobody to tell, and nothing more is
            // removed for nobody.
            Err(blit_core::deletion::DeletionError::Cancelled { .. }) => return Err(closed()),
            Err(error) => return Err(deletion_status(error)),
        }
    }
    Ok(entries_removed)
}

/// An entry's path below its operand as a transfer's manifest writes it
/// (cr-uv3-1): `/` between names, and the exact bytes of a name that is
/// not text — what a move's account holds a file by.
fn sent_path(relative_path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        relative_path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        wire_path(relative_path).into_bytes()
    }
}

/// Failures on their way to the asker, sent when what is held is large
/// enough or has waited long enough.
struct FailureBatch<'a> {
    failures: Vec<RmFailure>,
    /// The size of what is held, as it will be on the wire, near enough.
    bytes: usize,
    sent: std::time::Instant,
    sender: &'a RmSender,
}

impl FailureBatch<'_> {
    /// The most one message carries, by the SIZE of what is in it: a path
    /// can be thousands of bytes, and a message's limit is in bytes. A
    /// message is at most this and one failure more.
    const MOST_BYTES: usize = 64 * 1024;
    /// How long a failure may wait for the message to fill: a removal
    /// that fails seldom says so as it does.
    const WAIT: std::time::Duration = std::time::Duration::from_millis(50);

    fn add(&mut self, failure: RmFailure) -> Result<(), Stopped> {
        self.bytes += failure.relative_path.len()
            + failure.raw_relative_path.as_ref().map_or(0, Vec::len)
            + failure.reason.len()
            + 16;
        self.failures.push(failure);
        if self.bytes >= Self::MOST_BYTES || self.sent.elapsed() >= Self::WAIT {
            self.send()
        } else {
            Ok(())
        }
    }

    fn send(&mut self) -> Result<(), Stopped> {
        self.sent = std::time::Instant::now();
        if self.failures.is_empty() {
            return Ok(());
        }
        self.bytes = 0;
        let failures = std::mem::take(&mut self.failures);
        self.sender
            .blocking_send(Ok(RmReply {
                reply: Some(rm_reply::Reply::Failures(RmFailures { failures })),
            }))
            .map_err(|_| Stopped)
    }
}

fn deletion_status(error: blit_core::deletion::DeletionError) -> Status {
    match error {
        error @ blit_core::deletion::DeletionError::Containment { .. } => {
            Status::permission_denied(error.to_string())
        }
        error => internal_err("purge deletion failed", error),
    }
}

pub(crate) fn split_completion_prefix(raw: &str) -> Result<(PathBuf, String, String), Status> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok((PathBuf::from("."), String::new(), String::new()));
    }

    // Route user-typed input through the canonical helper so a literal
    // `\` in a POSIX filename (legal on macOS/Linux) is preserved, while
    // a Windows-native `Folder\file` is correctly split on the native
    // separator. Plain string `replace` would conflate the two.
    let normalized = blit_core::path_posix::relative_str_to_posix(trimmed);
    let (dir_part, leaf_part) = match normalized.rsplit_once('/') {
        Some((dir, leaf)) => (dir.to_string(), leaf.to_string()),
        None => (String::new(), normalized),
    };

    let display_prefix = dir_part.clone();
    let dir_rel = if dir_part.is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&dir_part)?
    };

    Ok((dir_rel, display_prefix, leaf_part))
}

pub(crate) fn list_completions(
    search_root: &Path,
    display_prefix: &str,
    leaf_prefix: &str,
    include_files: bool,
    include_dirs: bool,
) -> Result<Vec<String>, Status> {
    let mut results = Vec::new();
    let entries = fs::read_dir(search_root)
        .map_err(|err| io_to_status(format!("read_dir {}", search_root.display()), err))?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            io_to_status(format!("read_dir entry {}", search_root.display()), err)
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !leaf_prefix.is_empty() && !name.starts_with(leaf_prefix) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(err) => {
                log::warn!(
                    "failed to stat completion candidate {}: {}",
                    entry.path().display(),
                    err
                );
                continue;
            }
        };

        let is_dir = metadata.is_dir();
        if is_dir && !include_dirs {
            continue;
        }
        if !is_dir && !include_files {
            continue;
        }

        let mut completion = String::new();
        if !display_prefix.is_empty() {
            completion.push_str(display_prefix);
            if !display_prefix.ends_with('/') {
                completion.push('/');
            }
        }
        completion.push_str(&name);
        if is_dir {
            completion.push('/');
        }
        results.push(completion);
    }

    results.sort();
    results.dedup();
    Ok(results)
}

/// `du` over `start_rel` below the module
/// (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1): the walk is
/// `blit_core::disk_usage`, on this daemon's own disk; every line and
/// every unreadable entry goes to the asker as it is produced.
///
/// A start that cannot be read at all is one failure line for the start
/// itself, not a failed request: the asker prints it in `du`'s words
/// (`cannot access 'PATH': No such file or directory`) and exits 1, as
/// for any other entry.
pub(crate) fn stream_disk_usage(
    module_root: PathBuf,
    start: TypedOperand,
    options: UsageOptions,
    sender: &DiskUsageSender,
) -> Result<(), Status> {
    // As typed (cr-uv1-1): `du link/` measures the directory the link
    // points to, and `du doc/` of a file is `Not a directory`.
    // F2: containment check before any filesystem operation. The walk
    // never follows a link, but the start point itself can be a symlink
    // that points outside the module root. `module_root` here is the
    // canonical root — du callers don't mutate it.
    let readable = start_is_contained(&module_root, start.below(&module_root))?;

    let send = |reply: du_reply::Reply| {
        sender
            .blocking_send(Ok(DuReply { reply: Some(reply) }))
            .map_err(|_| Stopped)
    };
    let failure = |relative: &Path, action: &str, error: &std::io::Error| {
        du_reply::Reply::Failure(DuFailure {
            relative_path: wire_path(relative),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(relative),
            action: action.to_string(),
            reason: os_reason(error),
        })
    };
    let mut emit = |event: UsageEvent| match event {
        UsageEvent::Row(row) => send(du_reply::Reply::Row(DuRow {
            relative_path: wire_path(&row.relative_path),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(&row.relative_path),
            allocated_bytes: row.allocated_bytes,
        })),
        UsageEvent::Failure(failed) => {
            send(failure(&failed.relative_path, failed.action, &failed.error))
        }
    };
    let closed = || response_channel_closed("sending disk usage result");
    let start_abs = match readable {
        Readable::At(start_abs) => start_abs,
        Readable::Gone(error) => {
            return send(failure(Path::new(""), "cannot access", &error))
                .map_err(|Stopped| closed());
        }
    };
    match disk_usage(&start_abs, options, &mut emit) {
        Ok(()) => Ok(()),
        Err(UsageEnd::Stopped(Stopped)) => Err(closed()),
        Err(UsageEnd::Start(error)) => {
            send(failure(Path::new(""), "cannot access", &error)).map_err(|Stopped| closed())
        }
    }
}

/// `ls` of `start` below the module
/// (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 5): the listing is
/// `blit_core::listing`, on this daemon's own disk. What the path is goes
/// first; a directory's entries follow in batches, in the directory's own
/// order — the asker sorts.
///
/// A path that cannot be listed is one failure for the path itself, not a
/// failed request: the asker prints it in `ls`'s words and exits 2.
/// Nothing above the module is shown: its `..` is the module.
pub(crate) fn stream_listing(
    module_root: PathBuf,
    start: TypedOperand,
    options: blit_core::listing::ListOptions,
    sender: &super::LsSender,
) -> Result<(), Status> {
    use blit_core::generated::{
        ls_reply::Reply, LsDirectory, LsEntries, LsEntry, LsFacts, LsFailure, LsKind, LsReply,
    };
    use blit_core::listing::{list, Entry, Kind, ListEvent};

    /// The most entries one message carries.
    const MOST: usize = 512;

    // As typed (cr-uv1-1): `ls link/` lists the directory the link points
    // to, and `ls doc/` of a file is `Not a directory`.
    // F2: containment check before any filesystem operation. A path that
    // is itself a link out of the module is refused here, whether or not
    // it would be listed into. `module_root` is the canonical root.
    let readable = start_is_contained(&module_root, start.below(&module_root))?;

    let send = |reply: Reply| {
        sender
            .blocking_send(Ok(LsReply { reply: Some(reply) }))
            .map_err(|_| Stopped)
    };
    let wire = |entry: Entry| {
        let (name, raw_name) = text_and_bytes(&entry.name);
        LsEntry {
            name,
            raw_name,
            facts: entry.facts.map(|facts| {
                let (link_target, raw_link_target) = match &facts.link_target {
                    Some(target) => {
                        let (text, raw) = text_and_bytes(target.as_os_str());
                        (Some(text), raw)
                    }
                    None => (None, None),
                };
                LsFacts {
                    kind: (match facts.kind {
                        Kind::File => LsKind::File,
                        Kind::Directory => LsKind::Directory,
                        Kind::Link => LsKind::Link,
                        Kind::Fifo => LsKind::Fifo,
                        Kind::Socket => LsKind::Socket,
                        Kind::CharDevice => LsKind::CharDevice,
                        Kind::BlockDevice => LsKind::BlockDevice,
                        Kind::Other => LsKind::Other,
                    }) as i32,
                    size: facts.size,
                    mtime_seconds: facts.mtime_seconds,
                    allocated_bytes: facts.allocated_bytes,
                    link_target,
                    raw_link_target,
                }
            }),
        }
    };
    let failure = |action: &str, name: &std::ffi::OsStr, error: &std::io::Error| {
        let (name, raw_name) = text_and_bytes(name);
        Reply::Failure(LsFailure {
            action: action.to_string(),
            name,
            raw_name,
            reason: os_reason(error),
        })
    };
    let start_abs = match readable {
        Readable::At(start_abs) => start_abs,
        Readable::Gone(error) => {
            return send(failure("cannot access", std::ffi::OsStr::new(""), &error))
                .map_err(|Stopped| response_channel_closed("sending listing"));
        }
    };
    let mut batch: Vec<LsEntry> = Vec::new();
    let listed = {
        let mut emit = |event: ListEvent| match event {
            ListEvent::Itself(entry) => send(Reply::Itself(wire(entry))),
            ListEvent::Directory => send(Reply::Directory(LsDirectory {})),
            ListEvent::Entry(entry) => {
                batch.push(wire(entry));
                if batch.len() < MOST {
                    return Ok(());
                }
                send(Reply::Entries(LsEntries {
                    entries: std::mem::take(&mut batch),
                }))
            }
            ListEvent::Failure { name, error } => send(failure("cannot access", &name, &error)),
        };
        list(&start_abs, Some(&module_root), options, &mut emit)
    };
    let closed = || response_channel_closed("sending listing");
    if !batch.is_empty() {
        send(Reply::Entries(LsEntries { entries: batch })).map_err(|Stopped| closed())?;
    }
    match listed {
        Ok(()) => Ok(()),
        Err(end) => match end.told() {
            Some((action, error)) => {
                send(failure(action, std::ffi::OsStr::new(""), error)).map_err(|Stopped| closed())
            }
            None => Err(closed()),
        },
    }
}

/// A name as it goes to the asker: its text, and — when it is not valid
/// UTF-8, which a name on Unix need not be — its exact bytes beside it.
fn text_and_bytes(name: &std::ffi::OsStr) -> (String, Option<Vec<u8>>) {
    match name.to_str() {
        Some(text) => (text.to_string(), None),
        None => {
            #[cfg(unix)]
            let raw = {
                use std::os::unix::ffi::OsStrExt as _;
                Some(name.as_bytes().to_vec())
            };
            #[cfg(not(unix))]
            let raw = None;
            (name.to_string_lossy().into_owned(), raw)
        }
    }
}

/// A path below the start as it goes to the asker: `/`-separated text,
/// empty for the start itself.
fn wire_path(relative: &Path) -> String {
    if relative.as_os_str().is_empty() {
        String::new()
    } else {
        request_path_to_posix(relative)
    }
}

/// `find` from `start_rel` below the module
/// (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 3): the walk and its
/// tests are `blit_core::find`, on this daemon's own disk. The paths found
/// go to the asker in batches, in the walk's order; an entry that could
/// not be read goes at once, after what was found before it.
///
/// A start that is not there is one failure for the start itself, not a
/// failed request: the asker prints it in `find`'s words and exits 1, as
/// for any other entry.
///
/// `delete` (slice 4, `-delete`): what the tests hold for is REMOVED as
/// the walk gives it — contents before their directory — and nothing is
/// sent but what could not be: an entry that would not go, in the
/// system's words, the walk going on past it.
///
/// Nothing is removed THROUGH a link, wherever in the path typed it
/// stands (cr-uv4-3). A start that is a link typed with a separator after
/// it (`find link/ -delete`) is told in `find`'s words, with `find`'s
/// exit status, and what it points to is left whole, where `find` empties
/// it first. A start reached through a link (`find alias/entry -delete`)
/// is told in words of this command's own — `find` has none for it, and
/// removes through it — and nothing is walked.
pub(crate) fn stream_find(
    module_root: PathBuf,
    start: TypedOperand,
    start_name: Vec<u8>,
    options: FindOptions,
    delete: Option<DeleteFound>,
    sender: &FindSender,
) -> Result<(), Status> {
    // As typed (cr-uv1-1): `find link/` and `find link/.` start in the
    // directory the link points to, and `find doc/` of a file is
    // `Not a directory`.
    let start_as_directory = start.as_directory;
    // F2: containment check before any filesystem operation. The walk
    // follows no link below the start, and a start that is itself a link
    // out of the module is refused here, whether or not it would be
    // entered. `module_root` is the canonical root.
    let readable = start_is_contained(&module_root, start.below(&module_root))?;

    let mut found = FoundBatch {
        paths: Vec::with_capacity(FoundBatch::MOST),
        sent: std::time::Instant::now(),
        sender,
    };
    let failure = |relative: &Path, error: &std::io::Error, deleting: bool| {
        find_paths_reply::Reply::Failure(FindFailure {
            relative_path: wire_path(relative),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(relative),
            reason: os_reason(error),
            deleting,
        })
    };
    let start_abs = match readable {
        Readable::At(start_abs) => start_abs,
        Readable::Gone(error) => {
            return found
                .reply(failure(Path::new(""), &error, false))
                .map_err(|Stopped| response_channel_closed("sending find result"));
        }
    };
    // The start by its names alone, from the module's root: no separator
    // or `.` after it to make the system look through a link standing
    // there, and nothing resolved.
    let named: PathBuf = start_abs.components().collect();
    if let Some(asked) = &delete {
        // cr-uv4-3: a link ON THE WAY to the start. The system would look
        // through it for every entry the walk reads; nothing is removed
        // by a path that does.
        if let Some(link) = link_on_the_way(&module_root, &named) {
            let reason = format!(
                "its path goes through the symbolic link {}, and -delete removes nothing through \
                 a link: name the directory itself",
                blit_core::admin::quote::for_find(link.as_bytes())
            );
            return found
                .reply(failure(Path::new(""), &std::io::Error::other(reason), true))
                .map_err(|Stopped| response_channel_closed("sending find result"));
        }
        if start_as_directory
            && !asked.start_is_module
            && fs::symlink_metadata(&named).is_ok_and(|named| named.file_type().is_symlink())
            && fs::metadata(&named).is_ok_and(|target| target.is_dir())
        {
            // What `find` says of such a start once it has emptied the
            // directory: `Not a directory`, or `Invalid argument` for a
            // start ending in `.`. Here it is said with nothing removed.
            let reason = if asked.start_ends_in_dot {
                "Invalid argument"
            } else {
                "Not a directory"
            };
            return found
                .reply(failure(Path::new(""), &std::io::Error::other(reason), true))
                .map_err(|Stopped| response_channel_closed("sending find result"));
        }
    }
    let mut deleting =
        delete.map(|asked| Deleting::at(&module_root, &named, start_as_directory, asked));
    let walked = {
        let mut emit = |event: FindEvent| match event {
            // cr-uv4-2: nobody is waiting for this walk any more — the
            // asker went away, or was stopped — so it ends here. A walk
            // that finds nothing sends nothing, and one that removes what
            // it finds sends nothing for what went: neither would learn
            // of it from a send that fails.
            FindEvent::Walking if found.sender.is_closed() => Err(Stopped),
            FindEvent::Walking => Ok(()),
            FindEvent::Found(path, kind) => match &mut deleting {
                None => found.add(&path),
                // Asked again before EACH removal: what is removed after
                // the asker has gone is removed for nobody.
                Some(_) if found.sender.is_closed() => Err(Stopped),
                Some(deleting) => {
                    match deleting.remove(&path, kind == Some(blit_core::find::Kind::Directory)) {
                        Ok(()) => Ok(()),
                        Err(error) => found.reply(failure(&path, &error, true)),
                    }
                }
            },
            FindEvent::Failure(failed) => {
                found.send()?;
                found.reply(failure(&failed.relative_path, &failed.error, false))
            }
        };
        find(&start_abs, &start_name, &options, &mut emit)
    };
    let closed = || response_channel_closed("sending find result");
    match walked {
        Ok(()) => found.send().map_err(|Stopped| closed()),
        Err(FindEnd::Stopped(Stopped)) => Err(closed()),
        Err(FindEnd::Start(error)) => found
            .reply(failure(Path::new(""), &error, false))
            .map_err(|Stopped| closed()),
    }
}

/// What `find -delete` is told beyond the walk (slice 4).
pub(crate) struct DeleteFound {
    /// The start is the module itself, which is never removed.
    pub start_is_module: bool,
    /// The start was typed with `.` for its last name: `find t/. -delete`
    /// removes what is in `t`, and cannot remove `t/.`.
    pub start_ends_in_dot: bool,
}

/// The first link among the directories ON THE WAY from `root` to
/// `named` — every name of the path but its last — as the path below
/// `root` that reaches it. `None` when each is a directory, or is not
/// there (the walk says so itself).
fn link_on_the_way(root: &Path, named: &Path) -> Option<String> {
    let below = named.strip_prefix(root).ok()?;
    let mut at = root.to_path_buf();
    let mut names = below.components().peekable();
    while let Some(name) = names.next() {
        // The last name is the start itself: an entry, whatever it is.
        names.peek()?;
        at.push(name);
        if fs::symlink_metadata(&at).is_ok_and(|entry| entry.file_type().is_symlink()) {
            return Some(wire_path(at.strip_prefix(root).ok()?));
        }
    }
    None
}

/// `find -delete` on this daemon's disk: each entry found is removed by
/// its name in the directory that holds it (`blit_core::deletion::Remover`).
struct Deleting {
    remover: blit_core::deletion::Remover,
    /// Where the start is: its names as typed, below the module's root,
    /// with NOTHING resolved (cr-uv4-3). The remover opens each directory
    /// of a path in the one before it and never through a link, so a
    /// link anywhere in this path — put there while the walk runs, where
    /// none stood when the request was read — is an error for each entry
    /// below it and never a way through.
    start_at: PathBuf,
    /// The start was typed with a separator after it.
    start_as_directory: bool,
    asked: DeleteFound,
}

impl Deleting {
    /// To remove what a walk finds from the start at `named` — the
    /// start's names as typed, below `module_root`, which is the module's
    /// root as the daemon holds it: a path with no link in it
    /// (`runtime.rs` resolves it once, when the module is read).
    fn at(module_root: &Path, named: &Path, start_as_directory: bool, asked: DeleteFound) -> Self {
        Self {
            remover: blit_core::deletion::Remover::below(module_root),
            start_at: named.to_path_buf(),
            start_as_directory,
            asked,
        }
    }

    /// Remove the entry `relative` below the start (empty: the start).
    fn remove(&mut self, relative: &Path, directory: bool) -> std::io::Result<()> {
        if !relative.as_os_str().is_empty() {
            return self
                .remover
                .remove(&self.start_at.join(relative), directory);
        }
        // The module itself is never removed, and is passed over without
        // a word — as `find . -delete` passes over `.`, with everything
        // in it gone.
        if self.asked.start_is_module {
            return Ok(());
        }
        // `find t/. -delete`: what is in `t` has gone, and `t/.` is no
        // name a directory can be removed by — the system's own refusal,
        // and its words for it.
        if self.asked.start_ends_in_dot {
            return Err(std::io::Error::other("Invalid argument"));
        }
        // The start itself, by its name in the directory that holds it.
        // Typed with a separator after it, it goes as a directory or not
        // at all.
        let directory = directory || self.start_as_directory;
        self.remover.remove(&self.start_at, directory)
    }
}

/// The paths found and not yet sent.
struct FoundBatch<'a> {
    paths: Vec<FindPath>,
    sent: std::time::Instant,
    sender: &'a FindSender,
}

impl FoundBatch<'_> {
    /// The most paths one message carries.
    const MOST: usize = 512;
    /// How long a path found may wait for the batch to fill: a search
    /// that finds little shows what it finds as it finds it.
    const WAIT: std::time::Duration = std::time::Duration::from_millis(50);

    fn add(&mut self, relative: &Path) -> Result<(), Stopped> {
        self.paths.push(FindPath {
            relative_path: wire_path(relative),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(relative),
        });
        if self.paths.len() >= Self::MOST || self.sent.elapsed() >= Self::WAIT {
            self.send()
        } else {
            Ok(())
        }
    }

    fn send(&mut self) -> Result<(), Stopped> {
        self.sent = std::time::Instant::now();
        if self.paths.is_empty() {
            return Ok(());
        }
        let paths = std::mem::replace(&mut self.paths, Vec::with_capacity(Self::MOST));
        self.reply(find_paths_reply::Reply::Found(FindBatch { paths }))
    }

    fn reply(&self, reply: find_paths_reply::Reply) -> Result<(), Stopped> {
        self.sender
            .blocking_send(Ok(FindPathsReply { reply: Some(reply) }))
            .map_err(|_| Stopped)
    }
}

/// `df` for `path` (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1):
/// the operating system is asked about the filesystem that holds THIS
/// path. It once matched the path against a list of mounts and took the
/// longest match — a filesystem that list did not hold fell through to
/// the mount above it, and a module on a 16 GiB filesystem was reported
/// as another disk's 1.8 TiB.
///
/// A failure crosses with the operating system's words alone
/// (`No such file or directory`): the asker prints them after the path
/// it was given, as `df` does, and this daemon's own path stays here.
pub(crate) fn filesystem_space_for_path(path: &Path) -> Result<DfReply, Status> {
    match blit_core::disk_usage::filesystem_space(path) {
        Ok(space) => Ok(DfReply {
            total_bytes: space.total_bytes,
            used_bytes: space.used_bytes,
            available_bytes: space.available_bytes,
        }),
        Err(error) => {
            let reason = os_reason(&error);
            Err(match error.kind() {
                std::io::ErrorKind::NotFound => Status::not_found(reason),
                std::io::ErrorKind::PermissionDenied => Status::permission_denied(reason),
                _ => Status::internal(reason),
            })
        }
    }
}

#[cfg(test)]
mod rm_tests {
    //! The daemon's half of `rm`: what goes wrong with one operand is that
    //! operand's failure (cr-uv1-3). The removal itself is guarded in
    //! `blit_core::deletion`.

    use super::*;
    use std::fs;

    /// A removal's answer, held whole.
    pub(super) struct Answer {
        pub failures: Vec<RmFailure>,
        pub entries_removed: u64,
    }

    /// `paths` removed below `root`, every failure collected.
    pub(super) fn answer(root: &Path, paths: &[&str], recursive: bool, force: bool) -> Answer {
        let operands = read_operands(
            paths.iter().map(|path| path.to_string()).collect(),
            Vec::new(),
        )
        .expect("operands");
        let mut failures = Vec::new();
        let entries_removed = remove_each(
            root,
            root,
            &operands,
            blit_core::deletion::RmOptions { recursive, force },
            None,
            &|| false,
            &mut |failure| {
                failures.push(failure);
                Ok(())
            },
        )
        .expect("an answer for the request");
        Answer {
            failures,
            entries_removed,
        }
    }

    /// cr-uv3-1: a move's removal takes what its sessions sent, by the
    /// accounts they kept — this path, as the file it was — and the
    /// directories that leaves empty. A file no account holds (it came
    /// after the source was read), one that has changed since, a link and
    /// a file that is no regular file all stay, each told in the words
    /// for what it is; the directories that hold them stay untold. A
    /// plain removal — no account — takes everything, as it did.
    ///
    /// cr-uv4-1: "changed since" is the reviewer's case too — other
    /// contents of the same length, written 800 ms later in the same
    /// second (`same.txt`). And the accounts are of the path the sessions
    /// read: presented for another path, they let nothing go.
    /// cr-uv7-1: a later name of a file with several goes only if the
    /// account holds what the file holds: with the content the walk read,
    /// both names go; with other content, the later one stays.
    #[cfg(unix)]
    #[test]
    fn a_later_name_goes_only_with_the_content_that_was_sent() {
        use blit_core::moved::Account;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        for (as_read, left) in [(true, 0usize), (false, 1)] {
            let src = root.join("src");
            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("a"), "one file").unwrap();
            fs::hard_link(src.join("a"), src.join("b")).unwrap();
            let version =
                blit_core::moved::Version::of(&fs::symlink_metadata(src.join("a")).unwrap());
            let content =
                blit_core::moved::content_of(&mut fs::File::open(src.join("a")).unwrap()).unwrap();
            let recorded = if as_read { content } else { [0u8; 32] };
            let mut account = Account::of(&src);
            for name in ["a", "b"] {
                account.record(name.as_bytes(), &version);
                account.record_content(name.as_bytes(), &recorded);
            }
            let ticket = blit_core::moved::keep(account);
            blit_core::moved::confirm(&ticket, &[]);
            let accounts = blit_core::moved::held(&[ticket]).expect("kept");
            let operands = read_operands(vec!["src".to_string()], Vec::new()).expect("operands");
            remove_each(
                &root,
                &root,
                &operands,
                blit_core::deletion::RmOptions {
                    recursive: true,
                    force: true,
                },
                Some(&accounts),
                &|| false,
                &mut |_| Ok(()),
            )
            .unwrap();
            let stayed = fs::read_dir(&src)
                .map(|entries| entries.count())
                .unwrap_or(0);
            assert_eq!(stayed, left, "content as read: {as_read}");
            let _ = fs::remove_dir_all(&src);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_moves_removal_takes_what_its_sessions_sent() {
        use blit_core::moved::Account;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        let build = || {
            let src = root.join("src");
            for dir in ["sub/deep", "late", "empty/nested"] {
                fs::create_dir_all(src.join(dir)).unwrap();
            }
            for (name, body) in [
                ("a.txt", "sent"),
                ("b.txt", "came after the source was read"),
                ("grown.txt", "sent, then written to"),
                ("same.txt", "sent, as these very bytes"),
                ("sub/c.txt", "sent"),
                ("sub/deep/d.txt", "sent in another session"),
                ("late/new.txt", "came after"),
            ] {
                fs::write(src.join(name), body).unwrap();
            }
            std::os::unix::fs::symlink("a.txt", src.join("link")).unwrap();
            let piped = std::process::Command::new("mkfifo")
                .arg(src.join("pipe"))
                .status()
                .is_ok_and(|status| status.success());
            (src, piped)
        };
        let (src, piped) = build();
        let as_it_is = |name: &str| {
            blit_core::moved::Version::of(&fs::symlink_metadata(src.join(name)).unwrap())
        };
        let kept = |account: Account| {
            let ticket = blit_core::moved::keep(account);
            blit_core::moved::confirm(&ticket, &[]);
            ticket
        };
        // Written at a tenth of a second, and sent so.
        let tenth = filetime::FileTime::from_unix_time(1_800_000_000, 100_000_000);
        filetime::set_file_mtime(src.join("same.txt"), tenth).unwrap();
        let mut first = Account::of(&src);
        for name in ["a.txt", "sub/c.txt", "same.txt"] {
            first.record(name.as_bytes(), &as_it_is(name));
        }
        // Sent five bytes shorter than it now is.
        let grown = as_it_is("grown.txt");
        first.record(
            b"grown.txt",
            &blit_core::moved::Version {
                size: grown.size - 5,
                ..grown
            },
        );
        let mut second = Account::of(&src);
        second.record(b"sub/deep/d.txt", &as_it_is("sub/deep/d.txt"));
        let accounts = blit_core::moved::held(&[kept(first), kept(second)]).expect("both kept");
        // The reviewer's case: other bytes, as many, 800 ms later in the
        // same second.
        fs::write(src.join("same.txt"), "other bytes, just as many").unwrap();
        assert_eq!(
            fs::metadata(src.join("same.txt")).unwrap().len(),
            "sent, as these very bytes".len() as u64
        );
        let later = filetime::FileTime::from_unix_time(1_800_000_000, 900_000_000);
        filetime::set_file_mtime(src.join("same.txt"), later).unwrap();

        let remove = |sent: Option<&[std::sync::Arc<blit_core::moved::Closed>]>| {
            let operands = read_operands(vec!["src".to_string()], Vec::new()).expect("operands");
            let mut failures = Vec::new();
            let removed = remove_each(
                &root,
                &root,
                &operands,
                blit_core::deletion::RmOptions {
                    recursive: true,
                    force: true,
                },
                sent,
                &|| false,
                &mut |failure| {
                    failures.push((failure.relative_path, failure.reason));
                    Ok(())
                },
            )
            .expect("an answer");
            failures.sort();
            (removed, failures)
        };
        let (removed, failures) = remove(Some(&accounts));
        // a.txt, sub/c.txt, sub/deep/d.txt; then deep, sub, nested, empty.
        assert_eq!(removed, 7, "{failures:?}");
        let unsent = "it is not a file the move sent: it is new, or has changed since it was read";
        let mut expected = vec![
            ("b.txt".to_string(), unsent.to_string()),
            ("grown.txt".to_string(), unsent.to_string()),
            ("late/new.txt".to_string(), unsent.to_string()),
            (
                "link".to_string(),
                "it is a symbolic link, which a move does not carry".to_string(),
            ),
        ];
        if piped {
            expected.push((
                "pipe".to_string(),
                "it is not a regular file, and a move carries only those".to_string(),
            ));
        }
        expected.push(("same.txt".to_string(), unsent.to_string()));
        assert_eq!(failures, expected);
        for gone in ["a.txt", "sub", "empty"] {
            assert!(!src.join(gone).exists(), "{gone}");
        }
        for stays in ["b.txt", "grown.txt", "late/new.txt", "same.txt"] {
            assert!(src.join(stays).is_file(), "{stays}");
        }
        assert!(src.join("link").is_symlink());

        // A source that is itself one file is held under no name.
        let one = root.join("one.txt");
        fs::write(&one, "the source itself").unwrap();
        let meta = fs::metadata(&one).unwrap();
        let mut sole = Account::of(&one);
        sole.record(b"", &blit_core::moved::Version::of(&meta));
        let sole = blit_core::moved::held(&[kept(sole)]).unwrap();
        let alone = |name: &str| {
            let operands = read_operands(vec![name.to_string()], Vec::new()).expect("operands");
            let mut failures = Vec::new();
            let removed = remove_each(
                &root,
                &root,
                &operands,
                blit_core::deletion::RmOptions {
                    recursive: true,
                    force: true,
                },
                Some(&sole),
                &|| false,
                &mut |failure| {
                    failures.push(failure.reason);
                    Ok(())
                },
            )
            .expect("an answer");
            (removed, failures)
        };
        // cr-uv4-1: an account of `one.txt` says nothing of `two.txt`,
        // though it is this very file by another name — the same bytes,
        // times and file, linked there.
        fs::hard_link(&one, root.join("two.txt")).unwrap();
        assert_eq!(
            alone("two.txt"),
            (0, vec![NOT_THE_PATH_THAT_WAS_SENT.to_string()])
        );
        assert!(one.exists() && root.join("two.txt").exists());
        fs::remove_file(root.join("two.txt")).unwrap();
        // (A link made and unmade changes when the file was last changed:
        // it is told of again, as it now stands.)
        let mut again = Account::of(&one);
        again.record(
            b"",
            &blit_core::moved::Version::of(&fs::metadata(&one).unwrap()),
        );
        let sole = blit_core::moved::held(&[kept(again)]).unwrap();
        let alone = |name: &str| {
            let operands = read_operands(vec![name.to_string()], Vec::new()).expect("operands");
            let mut failures = Vec::new();
            let removed = remove_each(
                &root,
                &root,
                &operands,
                blit_core::deletion::RmOptions {
                    recursive: true,
                    force: true,
                },
                Some(&sole),
                &|| false,
                &mut |failure| {
                    failures.push(failure.reason);
                    Ok(())
                },
            )
            .expect("an answer");
            (removed, failures)
        };
        assert_eq!(alone("one.txt"), (1, Vec::new()));
        assert!(!one.exists());
        let _ = unsent;

        // A plain removal takes all of it.
        fs::remove_dir_all(&src).unwrap();
        let (src, piped) = build();
        let (removed, failures) = remove(None);
        assert!(failures.is_empty(), "{failures:?}");
        // Seven files, the link and the pipe, and six directories.
        assert_eq!(removed, 14 + u64::from(piped));
        assert!(!src.exists());
    }

    /// cr-uv1-5: what `stream_removal` SENDS. Every failure, in messages
    /// none of which outgrows the bound by more than one entry, and then
    /// — last — that it is done, with the count. 5,000 operands that are
    /// not there: five times the old limit of 1,000, and several
    /// messages' worth.
    #[test]
    fn a_removal_is_answered_in_bounded_messages_that_end_with_done() {
        use tokio::sync::mpsc;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        fs::write(root.join("there"), b"x").unwrap();
        let mut paths: Vec<String> = (0..5000).map(|at| format!("missing-{at:04}")).collect();
        paths.push("there".to_string());
        let operands = read_operands(paths.clone(), Vec::new()).expect("operands");
        let (tx, mut rx) = mpsc::channel::<Result<RmReply, Status>>(4096);
        let module = root.clone();
        let worker = std::thread::spawn(move || {
            stream_removal(
                module.clone(),
                module,
                operands,
                blit_core::deletion::RmOptions::default(),
                None,
                &tx,
            )
        });
        let mut replies = Vec::new();
        while let Some(reply) = rx.blocking_recv() {
            replies.push(reply.expect("a reply").reply.expect("failures or done"));
        }
        worker.join().unwrap().expect("the stream ends well");

        let Some(rm_reply::Reply::Done(done)) = replies.pop() else {
            panic!("the last message says the removal is done");
        };
        assert_eq!(done.entries_removed, 1);
        assert!(!root.join("there").exists());
        let mut told = Vec::new();
        for reply in &replies {
            let rm_reply::Reply::Failures(batch) = reply else {
                panic!("`done` comes once, and last");
            };
            let size: usize = batch
                .failures
                .iter()
                .map(|failure| failure.relative_path.len() + failure.reason.len() + 16)
                .sum();
            assert!(!batch.failures.is_empty());
            assert!(
                size <= FailureBatch::MOST_BYTES + 4096,
                "a message of {size} bytes"
            );
            told.extend(batch.failures.iter().map(|failure| failure.operand));
        }
        assert!(replies.len() > 1, "more than one message's worth");
        assert_eq!(told, (0..5000).collect::<Vec<u32>>(), "every one, in order");
    }

    /// cr-uv2-3: a removal whose asker has gone removes nothing more. The
    /// reviewer's case — the answer closed before the removal began: the
    /// file was removed all the same, and the closing found out only when
    /// `done` could not be sent.
    #[test]
    fn a_removal_nobody_waits_for_removes_nothing() {
        use tokio::sync::mpsc;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        fs::write(root.join("queued"), b"x").unwrap();
        fs::create_dir_all(root.join("tree/sub")).unwrap();
        fs::write(root.join("tree/sub/inner"), b"x").unwrap();
        let operands = read_operands(vec!["queued".into(), "tree".into()], Vec::new()).unwrap();
        let (tx, rx) = mpsc::channel::<Result<RmReply, Status>>(4);
        drop(rx);
        let status = stream_removal(
            root.clone(),
            root.clone(),
            operands,
            blit_core::deletion::RmOptions {
                recursive: true,
                force: false,
            },
            None,
            &tx,
        )
        .expect_err("nobody to answer");
        assert_eq!(status.code(), tonic::Code::Cancelled, "{status}");
        assert!(
            root.join("queued").exists() && root.join("tree/sub/inner").exists(),
            "nothing was removed for nobody"
        );
    }

    /// cr-uv2-3: and one who goes part way through stops it there — after
    /// the operand being removed, before the next.
    #[test]
    fn a_removal_stops_at_the_operand_where_its_asker_went() {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        for name in ["one", "two", "three"] {
            fs::write(root.join(name), b"x").unwrap();
        }
        let operands = read_operands(
            ["one", "two", "three"].map(String::from).to_vec(),
            Vec::new(),
        )
        .unwrap();
        // Asked as each operand is begun; gone from the second question.
        let asked = std::cell::Cell::new(0);
        let status = remove_each(
            &root,
            &root,
            &operands,
            blit_core::deletion::RmOptions::default(),
            None,
            &|| {
                asked.set(asked.get() + 1);
                asked.get() >= 2
            },
            &mut |failure| panic!("nothing fails here: {failure:?}"),
        )
        .expect_err("the asker went");
        assert_eq!(status.code(), tonic::Code::Cancelled, "{status}");
        assert!(
            !root.join("one").exists(),
            "what was asked for while it listened"
        );
        assert!(root.join("two").exists() && root.join("three").exists());
    }

    fn removed(root: &Path, paths: &[&str], recursive: bool) -> Answer {
        answer(root, paths, recursive, false)
    }

    fn failures(reply: &Answer) -> Vec<(u32, &str, &str)> {
        reply
            .failures
            .iter()
            .map(|failure| {
                (
                    failure.operand,
                    failure.relative_path.as_str(),
                    failure.reason.as_str(),
                )
            })
            .collect()
    }

    /// The reviewer's command: `first`, an entry below a directory that
    /// cannot be searched, `last`. The request used to end at the second
    /// with nothing reported — `first` deleted, `last` left, all three
    /// charged with the failure. Each operand now has its own answer.
    #[cfg(unix)]
    #[test]
    fn an_operand_that_cannot_be_reached_does_not_end_the_request() {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        fs::create_dir_all(root.join("blocked")).unwrap();
        for file in ["first", "blocked/entry", "last"] {
            fs::write(root.join(file), b"x").unwrap();
        }
        fs::set_permissions(root.join("blocked"), fs::Permissions::from_mode(0o000)).unwrap();
        let privileged = fs::read_dir(root.join("blocked")).is_ok();
        let reply = removed(&root, &["first", "blocked/entry", "last"], false);
        fs::set_permissions(root.join("blocked"), fs::Permissions::from_mode(0o755)).unwrap();
        if privileged {
            eprintln!("skipping: running privileged, mode 000 stops nothing");
            return;
        }
        assert_eq!(failures(&reply), [(1, "", "Permission denied")]);
        assert_eq!(reply.entries_removed, 2);
        assert!(!root.join("first").exists() && !root.join("last").exists());
        assert!(root.join("blocked/entry").exists());
    }

    /// An operand that cannot be one, and one that leaves the module
    /// through a link, are each a failure of their own; the operands
    /// around them are removed.
    #[test]
    fn an_operand_that_cannot_be_one_is_its_own_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let elsewhere = fs::canonicalize(outside.path()).unwrap();
        fs::write(elsewhere.join("victim"), b"x").unwrap();
        for file in ["first", "last"] {
            fs::write(root.join(file), b"x").unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&elsewhere, root.join("way-out")).unwrap();

        let reply = removed(&root, &["first", "../victim", "", ".", "last"], true);
        let found = failures(&reply);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(
            found[0].0 == 1 && found[0].2.contains("path not allowed"),
            "{found:?}"
        );
        assert_eq!(
            found[1..],
            [(2, "", THE_MODULE_ROOT), (3, "", THE_MODULE_ROOT)]
        );
        // A `..` as the last name is `rm`'s own refusal; one on the way is
        // a path this daemon does not take.
        assert_eq!(
            read_operands(vec!["x/..".into(), "x/../y".into()], Vec::new())
                .expect("operands")
                .into_iter()
                .map(|operand| operand.unwrap_err())
                .map(|reason| reason.starts_with("path not allowed"))
                .collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(reply.entries_removed, 2);
        assert!(!root.join("first").exists() && !root.join("last").exists());
        assert!(root.is_dir(), "the module's root stands");

        #[cfg(unix)]
        {
            fs::write(root.join("first"), b"x").unwrap();
            let reply = removed(&root, &["way-out/victim", "first"], true);
            assert_eq!(failures(&reply), [(0, "", "the path leaves the module")]);
            assert_eq!(reply.entries_removed, 1);
            assert!(elsewhere.join("victim").exists() && !root.join("first").exists());
        }
    }
}

#[cfg(test)]
mod typed_operand_tests {
    //! cr-uv1-1: the daemon acts on a path as it was typed.

    use super::*;
    use std::fs;

    /// The reviewer's three commands, as the request carries them. `work/.`
    /// was read as `work`, and removed with everything in it; `doc/` was
    /// read as `doc`, and the file unlinked; `report ` (a space after it)
    /// was read as `report`, and the wrong file went.
    #[test]
    fn an_operand_is_removed_as_it_was_typed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        let build = || {
            fs::create_dir_all(root.join("work/sub")).unwrap();
            for file in ["work/a", "work/sub/b", "doc", "report", "report "] {
                fs::write(root.join(file), b"x").unwrap();
            }
        };
        let removed = |paths: &[&str], recursive: bool, force: bool| {
            build();
            let reply = super::rm_tests::answer(&root, paths, recursive, force);
            let failures: Vec<(u32, String)> = reply
                .failures
                .iter()
                .map(|failure| (failure.operand, failure.reason.clone()))
                .collect();
            (reply.entries_removed, failures)
        };

        for dot in ["work/.", "work/..", "work/./", "work//.", "work/sub/.."] {
            assert_eq!(
                removed(&[dot, "doc"], true, true),
                (1, vec![(0, DOT_OR_DOT_DOT.to_string())]),
                "{dot}: refused, `-rf` or not, and the next operand removed"
            );
            assert!(root.join("work/sub/b").exists() && !root.join("doc").exists());
        }

        assert_eq!(
            removed(&["doc/"], false, false),
            (0, vec![(0, "Not a directory".to_string())])
        );
        assert!(root.join("doc").exists());
        // `-f` passes over it, as it does a missing operand; the file stays.
        assert_eq!(removed(&["doc/"], true, true), (0, vec![]));
        assert!(root.join("doc").exists());

        assert_eq!(removed(&["report "], false, false), (1, vec![]));
        assert!(root.join("report").exists() && !root.join("report ").exists());
        assert_eq!(
            removed(&[" report"], false, false),
            (0, vec![(0, "No such file or directory".to_string())])
        );

        // A directory typed as one, or with names on the way that say
        // nothing, is the directory.
        assert_eq!(
            removed(&["work/sub/", "work/./a"], true, false),
            (3, vec![])
        );
        assert!(root.join("work").is_dir() && !root.join("work/sub").exists());
    }

    /// Where a read starts, from the path as typed: the place, and
    /// whether it was typed as a directory — which is how the system is
    /// then asked.
    #[test]
    fn a_read_starts_where_the_path_as_typed_says() {
        let root = Path::new("/srv/module");
        for (typed, rel, as_directory, asked) in [
            // cr-uv5-5: the module itself is its root as it stands.
            ("", ".", false, "/srv/module"),
            (".", ".", true, "/srv/module/"),
            ("dir", "dir", false, "/srv/module/dir"),
            ("dir/", "dir", true, "/srv/module/dir/"),
            ("dir//", "dir", true, "/srv/module/dir/"),
            ("dir/.", "dir", true, "/srv/module/dir/"),
            ("a/./b", "a/b", false, "/srv/module/a/b"),
            ("report ", "report ", false, "/srv/module/report "),
        ] {
            let start = read_start(typed.as_bytes()).expect(typed);
            assert_eq!(
                (start.rel.as_path(), start.as_directory),
                (Path::new(rel), as_directory),
                "{typed:?}"
            );
            if cfg!(unix) {
                assert_eq!(start.below(root).as_os_str(), asked, "{typed:?}");
            }
        }
        assert!(read_start(b"dir/..").is_err() && read_start(b"../x").is_err());
    }

    /// cr-uv5-5: where a read starts is judged by the deepest part of it
    /// the system looks up. The module itself is in the module however it
    /// was typed, one file as much as a directory; a start past what
    /// cannot be looked up — here a link that leads round in a ring — is
    /// in the module when that is, and refused when the way to it leaves
    /// the module.
    #[cfg(unix)]
    #[test]
    fn a_start_is_judged_by_the_part_of_it_that_can_be_looked_up() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let top = tmp.path().canonicalize().unwrap();
        let (module, outside, one) = (top.join("module"), top.join("outside"), top.join("one.txt"));
        fs::create_dir_all(&module).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(&one, b"the whole module").unwrap();
        symlink("ring", module.join("ring")).unwrap();
        symlink("ring", outside.join("ring")).unwrap();
        symlink(&outside, module.join("out")).unwrap();
        let start = |typed: &str| read_start(typed.as_bytes()).expect(typed);

        // The module itself, a directory or one file, typed either way.
        for root in [&module, &one] {
            for typed in ["", "."] {
                let at = start(typed).below(root);
                assert!(
                    start_is_contained(root, at).is_ok(),
                    "{typed:?} of {root:?}"
                );
            }
        }
        // cr-uv6-2: a link put where the module's root was. The module
        // is then somewhere else, and neither it nor anything below it
        // is read — a listing of the root showed the other directory.
        let moved = top.join("was-the-module");
        fs::rename(&module, &moved).unwrap();
        symlink(&outside, &module).unwrap();
        for typed in ["", ".", "ring", "ring/x"] {
            let at = start(typed).below(&module);
            let refused = start_is_contained(&module, at).expect_err(typed);
            assert_eq!(refused.code(), tonic::Code::PermissionDenied, "{typed:?}");
            assert!(refused.message().contains("path containment"), "{typed:?}");
        }
        fs::remove_file(&module).unwrap();
        // cr-uv7-2: a link to nothing there is refused too; nothing at all
        // there is no way out — the read goes on, and says it is not there.
        symlink(top.join("nowhere"), &module).unwrap();
        for typed in ["", "."] {
            let at = start(typed).below(&module);
            assert!(start_is_contained(&module, at).is_err(), "{typed:?}");
        }
        fs::remove_file(&module).unwrap();
        // cr-uv8-2: and what is told is the error seen, the path not read.
        for typed in ["", ".", "ring/x"] {
            let at = start(typed).below(&module);
            assert!(
                matches!(
                    start_is_contained(&module, at),
                    Ok(Readable::Gone(ref error)) if error.kind() == std::io::ErrorKind::NotFound
                ),
                "{typed:?}"
            );
        }
        fs::rename(&moved, &module).unwrap();
        // Not looked up, in the module: the read goes on, and fails there.
        for typed in ["ring/x", "ring/x/y", "ring/", "missing/x"] {
            let at = start(typed).below(&module);
            assert!(start_is_contained(&module, at).is_ok(), "{typed:?}");
        }
        // By a way out of the module: refused, whatever cannot be looked
        // up beyond the link that leaves it.
        for typed in ["out", "out/", "out/ring/x", "out/missing"] {
            let at = start(typed).below(&module);
            let refused = start_is_contained(&module, at).expect_err(typed);
            assert_eq!(refused.code(), tonic::Code::PermissionDenied, "{typed:?}");
            assert!(refused.message().contains("path containment"), "{typed:?}");
        }
    }

    /// cr-uv1-4: a path whose bytes are not text is the name those bytes
    /// are — as a start, and as an operand of a removal — and the bytes
    /// win over the text beside them, which is only a near-copy.
    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_text_is_read_by_its_bytes() {
        use std::os::unix::ffi::OsStrExt as _;
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        let name = std::ffi::OsStr::from_bytes(b"bad\xff.txt");
        if fs::write(root.join(name), b"x").is_err() {
            eprintln!("this filesystem refuses a name that is not text");
            return;
        }
        fs::write(root.join("bad\u{fffd}.txt"), b"the near-copy").unwrap();

        let start = read_start(typed_bytes("bad\u{fffd}.txt", Some(b"bad\xff.txt"))).unwrap();
        assert_eq!(start.rel.as_os_str(), name);
        assert_eq!(typed_bytes("plain", None), b"plain");
        assert_eq!(typed_bytes("plain", Some(b"")), b"plain");

        let operands = read_operands(
            vec!["bad\u{fffd}.txt".into(), "missing".into()],
            vec![b"bad\xff.txt".to_vec(), Vec::new()],
        )
        .expect("operands");
        let mut failures = Vec::new();
        let removed = remove_each(
            &root,
            &root,
            &operands,
            blit_core::deletion::RmOptions::default(),
            None,
            &|| false,
            &mut |failure| {
                failures.push((failure.operand, failure.reason));
                Ok(())
            },
        )
        .expect("an answer");
        assert_eq!(removed, 1);
        assert_eq!(failures, [(1, "No such file or directory".to_string())]);
        assert!(!root.join(name).exists(), "the name the bytes are");
        assert!(root.join("bad\u{fffd}.txt").exists(), "not its near-copy");

        // One byte form for each path, or none.
        assert!(read_operands(vec!["a".into(), "b".into()], vec![Vec::new()]).is_err());
        // And a `..` is a `..` in bytes too.
        assert!(read_start(b"dir\xff/../x").is_err());
    }
}

#[cfg(test)]
mod disk_usage_tests {
    //! The daemon's half of `du` (UTILITY_VERBS_LOCAL_PARITY slice 1): what
    //! it SENDS. The walk itself is guarded in `blit_core::disk_usage`.

    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    /// Everything `stream_disk_usage` sends for `start` under `root`.
    fn sent(root: &Path, start: &str, options: UsageOptions) -> Vec<du_reply::Reply> {
        // Drain on this thread while a worker sends: both ends block.
        let (tx, mut rx) = mpsc::channel::<Result<DuReply, Status>>(4096);
        let (module_root, start) = (
            root.to_path_buf(),
            read_start(start.as_bytes()).expect("a start"),
        );
        let worker =
            std::thread::spawn(move || stream_disk_usage(module_root, start, options, &tx));
        let mut replies = Vec::new();
        while let Some(item) = rx.blocking_recv() {
            replies.push(item.expect("a reply").reply.expect("a row or a failure"));
        }
        worker.join().unwrap().expect("the stream ends well");
        replies
    }

    fn rows(replies: &[du_reply::Reply]) -> Vec<(String, u64)> {
        replies
            .iter()
            .filter_map(|reply| match reply {
                du_reply::Reply::Row(row) => Some((row.relative_path.clone(), row.allocated_bytes)),
                du_reply::Reply::Failure(_) => None,
            })
            .collect()
    }

    /// 3 folders, each holding files 3 levels deep: 180 files.
    fn tree() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempdir().unwrap();
        // Canonicalize: on macOS the temp dir is under a /var →
        // /private/var symlink, and the containment check compares the
        // canonical start path against the module root. The real daemon
        // always passes a canonical module root.
        let root = fs::canonicalize(tmp.path()).unwrap();
        for child_name in ["alpha", "beta", "gamma"] {
            let child = root.join(child_name);
            fs::create_dir_all(child.join("deep/deeper")).unwrap();
            for i in 0..20 {
                fs::write(child.join(format!("f{i}.bin")), [0u8; 100]).unwrap();
                fs::write(child.join("deep").join(format!("g{i}.bin")), [0u8; 100]).unwrap();
                fs::write(
                    child.join("deep/deeper").join(format!("h{i}.bin")),
                    [0u8; 100],
                )
                .unwrap();
            }
        }
        (tmp, root)
    }

    /// AC-7: `du` without `-a` sends NO file rows — a row is a line that
    /// will be printed, and nothing else crosses. 180 files, 10 rows: the
    /// module, its 3 folders and their 6 sub-folders, the start LAST and
    /// carrying everything. With `-a`, the 180 files are rows too.
    #[test]
    fn du_sends_a_row_for_each_line_it_prints_and_no_other() {
        let (_tmp, root) = tree();
        let every_level = UsageOptions::default();
        let sent_rows = rows(&sent(&root, ".", every_level));
        assert_eq!(sent_rows.len(), 10, "{sent_rows:?}");
        assert!(
            sent_rows.iter().all(|(path, _)| !path.ends_with(".bin")),
            "no file row without -a: {sent_rows:?}"
        );
        let (start, total) = sent_rows.last().unwrap();
        assert_eq!(
            start, "",
            "the start is the last row, with no path of its own"
        );
        assert!(
            sent_rows.iter().all(|(_, bytes)| bytes <= total),
            "the start carries every entry: {sent_rows:?}"
        );
        // An entry after everything below it.
        let at = |path: &str| sent_rows.iter().position(|(p, _)| p == path).unwrap();
        assert!(at("alpha/deep/deeper") < at("alpha/deep") && at("alpha/deep") < at("alpha"));

        let all = rows(&sent(
            &root,
            ".",
            UsageOptions {
                all_files: true,
                max_depth: None,
            },
        ));
        assert_eq!(all.len(), 190, "10 directories and 180 files");
    }

    /// AC-2: `-d N` counts from the START path, not from the module, and
    /// bounds the rows without changing a total. `-d 1` under `alpha` is
    /// `alpha/deep` and `alpha` itself — the old request counted from the
    /// module's root, so a depth of 1 on a sub-folder printed only that
    /// folder — and `-d 0` (`-s`) is the start alone.
    #[test]
    fn du_depth_counts_from_the_start_and_bounds_rows_not_totals() {
        let (_tmp, root) = tree();
        let unbounded = rows(&sent(&root, "alpha", UsageOptions::default()));
        let total = unbounded.last().unwrap().1;
        let paths = |replies: &[(String, u64)]| -> Vec<String> {
            replies.iter().map(|(path, _)| path.clone()).collect()
        };
        assert_eq!(paths(&unbounded), ["deep/deeper", "deep", ""]);

        let depth = |max| UsageOptions {
            all_files: false,
            max_depth: Some(max),
        };
        let one = rows(&sent(&root, "alpha", depth(1)));
        assert_eq!(paths(&one), ["deep", ""]);
        assert_eq!(one.last().unwrap().1, total, "a bound changes no total");
        let summary = rows(&sent(&root, "alpha", depth(0)));
        assert_eq!(summary, [(String::new(), total)]);
        let module = rows(&sent(&root, ".", depth(1)));
        assert_eq!(module.len(), 4, "the module and its 3 folders: {module:?}");
    }

    /// AC-2, AC-12: a file given as the start is its own row; a start that
    /// does not exist is one failure for the start, in `du`'s words, and
    /// the stream still ends well.
    #[test]
    fn du_answers_for_a_file_and_for_a_start_that_is_not_there() {
        let (_tmp, root) = tree();
        let file = sent(&root, "alpha/f0.bin", UsageOptions::default());
        assert_eq!(rows(&file).len(), 1);
        assert_eq!(rows(&file)[0].0, "");

        let missing = sent(&root, "no-such-entry", UsageOptions::default());
        match missing.as_slice() {
            [du_reply::Reply::Failure(failure)] => {
                assert_eq!(failure.relative_path, "");
                assert_eq!(failure.action, "cannot access");
                assert!(
                    !failure.reason.contains("os error") && !failure.reason.is_empty(),
                    "the system's words alone: {}",
                    failure.reason
                );
            }
            other => panic!("one failure for the start, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod status_mapping_tests {
    use super::*;
    use tempfile::tempdir;
    use tonic::Code;

    #[test]
    fn missing_filesystem_paths_cross_as_not_found() {
        let temp = tempdir().expect("tempdir");
        let missing = temp.path().join("missing");

        let completion_error =
            list_completions(&missing, "", "", true, true).expect_err("missing completion root");
        assert_eq!(completion_error.code(), Code::NotFound);

        let space_error =
            filesystem_space_for_path(&missing).expect_err("a path that is not there");
        assert_eq!(space_error.code(), Code::NotFound);
        assert!(
            !space_error.message().contains(&*missing.to_string_lossy())
                && !space_error.message().contains("os error"),
            "the system's words alone, and no path of this daemon's: {}",
            space_error.message()
        );
        // The filesystem that holds a path that IS there has a size.
        let space = filesystem_space_for_path(temp.path()).expect("the temp folder's filesystem");
        assert!(space.total_bytes > 0 && space.used_bytes <= space.total_bytes);
    }

    #[test]
    fn closed_admin_streams_cross_as_cancelled_with_one_vocabulary() {
        let temp = tempdir().expect("tempdir");
        let file = temp.path().join("entry");
        std::fs::File::create(&file).expect("create empty fixture");
        let module_root = std::fs::canonicalize(temp.path()).expect("canonical tempdir");

        let (disk_sender, disk_receiver) = tokio::sync::mpsc::channel(1);
        drop(disk_receiver);
        let disk_error = stream_disk_usage(
            module_root.clone(),
            read_start(b"entry").expect("a start"),
            UsageOptions::default(),
            &disk_sender,
        )
        .expect_err("closed disk-usage stream");
        assert_eq!(disk_error.code(), Code::Cancelled);
        assert_eq!(
            disk_error.message(),
            "response channel closed (peer disconnected): sending disk usage result"
        );

        let (find_sender, find_receiver) = tokio::sync::mpsc::channel(1);
        drop(find_receiver);
        let find_error = stream_find(
            module_root,
            read_start(b"entry").expect("a start"),
            b"entry".to_vec(),
            FindOptions::default(),
            None,
            &find_sender,
        )
        .expect_err("closed find stream");
        assert_eq!(find_error.code(), Code::Cancelled);
        assert_eq!(
            find_error.message(),
            "response channel closed (peer disconnected): sending find result"
        );
    }

    /// cr-uv4-2: a walk nobody is waiting for stops. With the asker gone
    /// before it began, `find -delete` removes nothing — a removal that
    /// succeeds sends nothing, so no failed send would have told it —
    /// and a walk that finds nothing does not go on to its end.
    #[test]
    fn find_stops_when_its_asker_has_gone() {
        let temp = tempdir().expect("tempdir");
        let module_root = std::fs::canonicalize(temp.path()).expect("canonical tempdir");
        for dir in ["t/a", "t/b/c"] {
            std::fs::create_dir_all(module_root.join(dir)).unwrap();
        }
        let files = [
            "t/one.tmp",
            "t/a/two.tmp",
            "t/b/c/three.tmp",
            "t/b/keep.txt",
        ];
        for file in files {
            std::fs::write(module_root.join(file), b"x").unwrap();
        }
        let gone = || {
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            drop(receiver);
            sender
        };
        let named = |pattern: &[u8]| FindOptions {
            names: vec![blit_core::find_name::NamePattern::new(pattern, false).unwrap()],
            ..FindOptions::default()
        };
        let deleting = || DeleteFound {
            start_is_module: false,
            start_ends_in_dot: false,
        };
        // Everything found would be removed, in silence.
        for options in [
            FindOptions {
                contents_first: true,
                ..FindOptions::default()
            },
            FindOptions {
                contents_first: true,
                ..named(b"*.tmp")
            },
        ] {
            let ended = stream_find(
                module_root.clone(),
                read_start(b"t").expect("a start"),
                b"t".to_vec(),
                options,
                Some(deleting()),
                &gone(),
            )
            .expect_err("nobody is waiting");
            assert_eq!(ended.code(), Code::Cancelled);
            for file in files {
                assert!(module_root.join(file).exists(), "{file} was removed");
            }
        }
        // A start that is a file is found before any directory is read:
        // the walk has had no word with its caller, and the removal asks.
        let ended = stream_find(
            module_root.clone(),
            read_start(b"t/one.tmp").expect("a start"),
            b"one.tmp".to_vec(),
            FindOptions {
                contents_first: true,
                ..FindOptions::default()
            },
            Some(deleting()),
            &gone(),
        )
        .expect_err("nobody is waiting");
        assert_eq!(ended.code(), Code::Cancelled);
        assert!(module_root.join("t/one.tmp").exists());
        // A walk that finds nothing has nothing to send, and stops too.
        let ended = stream_find(
            module_root.clone(),
            read_start(b"t").expect("a start"),
            b"t".to_vec(),
            named(b"no such name"),
            None,
            &gone(),
        )
        .expect_err("nobody is waiting");
        assert_eq!(ended.code(), Code::Cancelled);
        // With its asker there, the same request removes what it found.
        let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
        stream_find(
            module_root.clone(),
            read_start(b"t").expect("a start"),
            b"t".to_vec(),
            FindOptions {
                contents_first: true,
                ..named(b"*.tmp")
            },
            Some(deleting()),
            &sender,
        )
        .expect("the walk");
        assert!(receiver.try_recv().is_err(), "nothing is said of what went");
        assert!(!module_root.join("t/b/c/three.tmp").exists());
        assert!(module_root.join("t/b/keep.txt").exists());
    }

    /// Slice 4: nothing goes THROUGH a link. A start that is a link typed
    /// with a separator after it is told before the walk; were a link put
    /// where the start was while the walk runs, what was found below it
    /// still does not go from the directory the link points to, and the
    /// link is not removed for the directory that was typed.
    #[cfg(unix)]
    #[test]
    fn find_delete_removes_nothing_through_a_link_standing_where_the_start_was() {
        let temp = tempdir().expect("tempdir");
        let root = std::fs::canonicalize(temp.path()).expect("canonical tempdir");
        std::fs::create_dir_all(root.join("t/real")).unwrap();
        std::fs::write(root.join("t/real/f"), b"x").unwrap();
        std::os::unix::fs::symlink("real", root.join("t/start")).unwrap();
        let deleting = |start_at: PathBuf, start_as_directory: bool| {
            Deleting::at(
                &root,
                &start_at,
                start_as_directory,
                DeleteFound {
                    start_is_module: false,
                    start_ends_in_dot: false,
                },
            )
        };
        // cr-uv4-3: and a link ON THE WAY to the start — put there after
        // the request was read — is no way through either.
        std::fs::create_dir(root.join("t/real/deep")).unwrap();
        std::fs::write(root.join("t/real/deep/g"), b"x").unwrap();
        let mut by_a_link = deleting(root.join("t/start/deep"), false);
        by_a_link
            .remove(Path::new("g"), false)
            .expect_err("nothing goes by a path through a link");
        by_a_link
            .remove(Path::new(""), true)
            .expect_err("nor the start itself");
        assert!(root.join("t/real/deep/g").exists());
        std::fs::remove_dir_all(root.join("t/real/deep")).unwrap();
        assert_eq!(
            link_on_the_way(&root, &root.join("t/start/deep")).as_deref(),
            Some("t/start")
        );
        // The start's own name is not on the way to it, and what is not
        // there is no link.
        assert_eq!(link_on_the_way(&root, &root.join("t/start")), None);
        assert_eq!(link_on_the_way(&root, &root.join("t/real/f")), None);
        assert_eq!(link_on_the_way(&root, &root.join("t/missing/x/y")), None);
        assert_eq!(link_on_the_way(&root, &root), None);

        let mut through = deleting(root.join("t/start"), true);
        through
            .remove(Path::new("f"), false)
            .expect_err("nothing goes through the link");
        assert!(root.join("t/real/f").exists());
        // The start was typed as a directory: what stands there now goes
        // as one, or not at all.
        through
            .remove(Path::new(""), false)
            .expect_err("a link is no directory");
        assert!(root.join("t/start").is_symlink());

        // A start that IS the directory loses what was found in it, then
        // goes itself.
        let mut plain = deleting(root.join("t/real"), true);
        plain
            .remove(Path::new("f"), false)
            .expect("the entry found");
        assert!(!root.join("t/real/f").exists());
        plain.remove(Path::new(""), true).expect("the start");
        assert!(!root.join("t/real").exists());
        // And a link typed plainly goes as the link.
        std::fs::create_dir(root.join("t/real")).unwrap();
        let mut link = deleting(root.join("t/start"), false);
        link.remove(Path::new(""), false).expect("the link");
        assert!(!root.join("t/start").is_symlink() && root.join("t/real").is_dir());
    }
}
