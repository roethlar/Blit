//! What a move's session sent, kept by the end that sent it (cr-uv3-1,
//! cr-uv4-1, `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`).
//!
//! A move from a daemon copies, and then asks the daemon to remove the
//! source. That removal was a recursive `rm` of whatever stood at the
//! source by then — so a link, which a transfer does not carry, went with
//! the files that were copied, and so would a file that arrived after the
//! source had been read. Nothing tied the removal to the transfer.
//!
//! The end that is read from keeps an ACCOUNT of the files a session put
//! on its manifest, when the session was opened as one whose source will
//! be removed, and hands the other end a TICKET for it. The removal
//! presents the tickets of the move's sessions, and takes a file only if
//! one of those accounts holds it.
//!
//! An account holds a file as THE FILE IT WAS ([`Version`]): its size, the
//! time it was last written to the nanosecond, and — where the system
//! keeps them — the time it was last changed in any way and which file it
//! is. The removal finds that again, whole, or leaves the file (cr-uv4-1:
//! held by its size and whole second, a file rewritten with as many bytes
//! within the same second passed for the one that was sent).
//!
//! And the file it was is the file THE WALK SAW ([`Sightings`],
//! cr-uv5-2): the look its size and time on the manifest came from,
//! before a byte of it was read or hashed. Whatever is done to it after
//! that look, whenever, the removal finds another file.
//!
//! An account is for ONE PLACE — the path its session read — and answers
//! a removal of that path alone.
//!
//! And an account answers nothing until the session it was written in has
//! ENDED WELL: the end that received says what did not land, those paths
//! are struck from the account, and only then is it [`confirm`]ed. The
//! ticket is given when the manifest is complete, before a byte of any
//! file has gone; a session that faults, is cancelled, or ends without
//! that word leaves an account no removal can use.
//!
//! An account is a sorted list of 128-bit digests, held in this process: a
//! daemon that restarts forgets them, and the removal is then refused
//! whole, with the source as it was.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// A ticket is this many bytes.
pub const TICKET_BYTES: usize = 16;

/// How long an account is kept: a move's retries, with their waits, end
/// well inside it.
const KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// What a process keeps of moves, at most (cr-uv4-5).
#[derive(Debug, Clone, Copy)]
struct Bounds {
    /// The files all kept accounts hold between them. Past it the oldest
    /// account goes, and a removal that presents its ticket is refused.
    /// An account that by itself holds more is not kept at all.
    files: usize,
    /// The accounts kept, however few files each holds: a ticket costs
    /// something even for a session that sent nothing.
    accounts: usize,
}

/// 64 million files — two gibibytes of digests while their sessions are
/// open, one once they have ended — in at most 4,096 accounts.
const BOUNDS: Bounds = Bounds {
    files: 64 * 1024 * 1024,
    accounts: 4096,
};

/// A file as it stood when it was looked at: what a removal must find
/// again, unchanged, to take it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub size: u64,
    /// When it was last written: nanoseconds since 1970 (before it, less
    /// than none), as the filesystem keeps it.
    pub mtime_nanos: i128,
    /// When it was last changed in any way — written, renamed over, its
    /// attributes set — where the system keeps that (Unix); 0 elsewhere.
    /// No program sets this one: it moves with every change.
    pub ctime_nanos: i128,
    /// Which file it is, where the system says (Unix: its device and its
    /// number there); 0 elsewhere.
    pub device: u64,
    pub inode: u64,
}

impl Version {
    /// The version `metadata` describes — the entry itself, never what a
    /// link points to.
    pub fn of(metadata: &std::fs::Metadata) -> Self {
        let mtime_nanos = metadata.modified().map_or(0, crate::find::nanos_of);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            Self {
                size: metadata.len(),
                mtime_nanos,
                ctime_nanos: i128::from(metadata.ctime()) * 1_000_000_000
                    + i128::from(metadata.ctime_nsec()),
                device: metadata.dev(),
                inode: metadata.ino(),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                size: metadata.len(),
                mtime_nanos,
                ctime_nanos: 0,
                device: 0,
                inode: 0,
            }
        }
    }
}

fn first_sixteen(hasher: &blake3::Hasher) -> u128 {
    let mut first = [0u8; 16];
    first.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    u128::from_le_bytes(first)
}

fn path_digest(path: &[u8]) -> u128 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"path");
    hasher.update(path);
    first_sixteen(&hasher)
}

/// cr-uv7-1, cr-uv8-1: the digest of what `file` holds — read through
/// that descriptor, AS THE FILE `version` IS: that file (Unix: its device
/// and its number there), unchanged. Anything else is an error, and
/// nothing is known of what it holds.
///
/// The descriptor is looked at before a byte of it is read (cr-uv9-2):
/// another file, or this one changed already, is not read at all — not
/// for as long as the file that was seen. No more than `version.size`
/// bytes are read, and once they have been it is looked at again: the
/// time a file was last changed (Unix) moves with every change to it, and
/// no program sets it, so a descriptor that is the version once read was
/// that file, unchanged, to the end of the read — written to anywhere
/// meanwhile, a part already read included, it is not.
pub fn content_of(file: &std::fs::File, version: &Version) -> std::io::Result<[u8; 32]> {
    use std::io::Read as _;
    let as_seen = || -> std::io::Result<()> {
        if Version::of(&file.metadata()?) == *version {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "not the file that was seen: another file, or changed since",
            ))
        }
    };
    as_seen()?;
    while_content_is_read(version, "reading", 0);
    let mut hasher = blake3::Hasher::new();
    let read = std::io::copy(&mut file.take(version.size), &mut hasher)?;
    while_content_is_read(version, "read", read);
    if read != version.size {
        return Err(std::io::Error::other(
            "not the file that was seen: it holds less than it did",
        ));
    }
    as_seen()?;
    Ok(*hasher.finalize().as_bytes())
}

/// A moment in a read of what a file holds, for a test to change the file
/// in: `stage` says which, `read` how many bytes have been read.
fn while_content_is_read(_version: &Version, _stage: &'static str, _read: u64) {
    #[cfg(test)]
    {
        let hooks: Vec<ContentReadHook> = WHILE_CONTENT_IS_READ_HOOKS
            .lock()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(file, _)| *file == (_version.device, _version.inode))
                    .map(|(_, hook)| Arc::clone(hook))
                    .collect()
            })
            .unwrap_or_default();
        for hook in hooks {
            hook(_stage, _read);
        }
    }
}

#[cfg(test)]
pub(crate) type ContentReadHook = Arc<dyn Fn(&'static str, u64) + Send + Sync>;

/// Tests' hooks on a read of what a file holds, by the file it is read as
/// (its device and its number).
#[cfg(test)]
pub(crate) static WHILE_CONTENT_IS_READ_HOOKS: Mutex<Vec<((u64, u64), ContentReadHook)>> =
    Mutex::new(Vec::new());

fn content_digest(path: &[u8], content: &[u8; 32]) -> u128 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"content");
    hasher.update(&(path.len() as u64).to_le_bytes());
    hasher.update(path);
    hasher.update(content);
    first_sixteen(&hasher)
}

fn digest(path: &[u8], version: &Version) -> u128 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(path.len() as u64).to_le_bytes());
    hasher.update(path);
    hasher.update(&version.size.to_le_bytes());
    hasher.update(&version.mtime_nanos.to_le_bytes());
    hasher.update(&version.ctime_nanos.to_le_bytes());
    hasher.update(&version.device.to_le_bytes());
    hasher.update(&version.inode.to_le_bytes());
    first_sixteen(&hasher)
}

/// A path with nothing after its last name and no `.` in it: two
/// spellings of one place compare equal. Nothing is resolved.
fn place(path: &Path) -> PathBuf {
    path.components().collect()
}

/// One session's account, as it is written.
#[derive(Debug)]
pub struct Account {
    root: PathBuf,
    /// Each file: its path's digest, to strike it by, and the digest of
    /// the path with its version, which is what the account holds.
    sent: Vec<(u128, u128)>,
    /// cr-uv7-1: and for a file with several names, its path's digest and
    /// the digest of the path with the file's CONTENT, as the walk read it.
    contents: Vec<(u128, u128)>,
}

impl Account {
    /// An account of what a session read from `root` — the path the
    /// session was opened on, on this machine.
    pub fn of(root: &Path) -> Self {
        Self {
            root: place(root),
            sent: Vec::new(),
            contents: Vec::new(),
        }
    }

    /// cr-uv7-1: what a file the session put on its manifest held, read
    /// as the walk saw it — kept for a file with several names, whose later
    /// names a removal can take only by its content.
    pub fn record_content(&mut self, path: &[u8], content: &[u8; 32]) {
        self.contents
            .push((path_digest(path), content_digest(path, content)));
    }

    /// A file the session put on its manifest: its path below the source
    /// (`/` between names, the exact bytes of a name that is not text;
    /// empty for a source that is itself the file), as it stood.
    pub fn record(&mut self, path: &[u8], version: &Version) {
        self.sent.push((path_digest(path), digest(path, version)));
    }

    pub fn len(&self) -> usize {
        self.sent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sent.is_empty()
    }
}

/// What a walk saw of the files it put on a manifest, each AS THE WALK'S
/// OWN LOOK FOUND IT (cr-uv5-2) — the look its size and its time on the
/// manifest were taken from, before anything of the file was read: not
/// its bytes, not its checksum.
///
/// A move's account is written from these. It was written from a second
/// look, at the manifest's end, which took any file still of the size and
/// the second the manifest wrote: a file rewritten after it had been read
/// or hashed — as many bytes, within that second — went into the account
/// as rewritten, and the removal took bytes that had been sent to nobody.
/// Held as the first look found it, a file changed in any way since is
/// another file to the removal, whenever the change came.
///
/// ONE WALK'S (cr-uv6-4): a scan that records them makes its own, and
/// hands it back with the scan (`SourceScan::sightings`). They were kept
/// on the source, which more than one scan can share: a second walk
/// cleared the first one's record, or wrote over it, and one move's
/// account could then hold a file as ANOTHER walk had seen it.
#[derive(Debug, Default)]
pub struct Sightings {
    /// By a name's digest: the digest of the name with the file as seen.
    seen: Mutex<HashMap<u128, u128>>,
    /// cr-uv7-1: by a name's digest, the digest of the name with what the
    /// file held — for a file with several names.
    contents: Mutex<HashMap<u128, u128>>,
}

impl Sightings {
    /// What a walk saw, each as [`sighting`] makes one.
    pub fn publish(&self, seen: Vec<(u128, u128)>) {
        self.seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(seen);
    }

    /// The account of the files a session put on its manifest from `root`
    /// — `listed`, each by its [`name_key`] — every one as the walk saw
    /// it. A file the walk saw and the manifest does not list is not in
    /// it, and neither is a name the walk recorded nothing for: both stay
    /// where they are.
    pub fn account(&self, root: &Path, listed: &[u128]) -> Account {
        let seen = self
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let contents = self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let held = |map: &HashMap<u128, u128>| {
            listed
                .iter()
                .filter_map(|name| map.get(name).map(|whole| (*name, *whole)))
                .collect()
        };
        Account {
            root: place(root),
            sent: held(&seen),
            contents: held(&contents),
        }
    }

    /// cr-uv7-1: what a walk read of files with several names, each as
    /// [`content_sighting`] makes one.
    pub fn publish_contents(&self, contents: Vec<(u128, u128)>) {
        self.contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(contents);
    }
}

/// cr-uv7-1: what a walk read of one file with several names, for
/// [`Sightings::publish_contents`].
pub fn content_sighting(path: &[u8], content: &[u8; 32]) -> (u128, u128) {
    (path_digest(path), content_digest(path, content))
}

/// One file as a walk saw it, for [`Sightings::publish`]: its path below
/// the source, as [`Account::record`] takes one, and the file.
pub fn sighting(path: &[u8], version: &Version) -> (u128, u128) {
    (path_digest(path), digest(path, version))
}

/// What a file on a manifest is listed by, for [`Sightings::account`]:
/// its path below the source, as [`Account::record`] takes one.
pub fn name_key(path: &[u8]) -> u128 {
    path_digest(path)
}

/// A session's account, closed: the session ended well, and what did not
/// land has been struck from it.
#[derive(Debug)]
pub struct Closed {
    root: PathBuf,
    sent: Vec<u128>,
    contents: Vec<u128>,
}

impl Closed {
    /// The place this account is of: the path its session read.
    pub fn is_of(&self, root: &Path) -> bool {
        self.root == place(root)
    }

    /// Whether the session sent this file: this path, as this version. A
    /// file written to since, or put there since, is another.
    pub fn holds(&self, path: &[u8], version: &Version) -> bool {
        self.sent.binary_search(&digest(path, version)).is_ok()
    }

    /// cr-uv7-1: whether the session sent this path holding exactly this
    /// content — asked of a later name of a file with several, which no
    /// look at the file can tell from one rewritten in place.
    pub fn holds_content(&self, path: &[u8], content: &[u8; 32]) -> bool {
        self.contents
            .binary_search(&content_digest(path, content))
            .is_ok()
    }

    pub fn len(&self) -> usize {
        self.sent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sent.is_empty()
    }
}

enum State {
    /// Written, and its session not yet ended: it answers nothing.
    Open(Account),
    Confirmed(Arc<Closed>),
}

impl State {
    fn len(&self) -> usize {
        match self {
            Self::Open(account) => account.len(),
            Self::Confirmed(account) => account.len(),
        }
    }
}

struct Kept {
    state: State,
    since: Instant,
    /// The order accounts were kept in: the least is the oldest.
    order: u64,
}

fn kept() -> &'static Mutex<HashMap<[u8; TICKET_BYTES], Kept>> {
    static KEPT: OnceLock<Mutex<HashMap<[u8; TICKET_BYTES], Kept>>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
}

/// A ticket nothing else in this process has: the standard library's
/// per-process random keys, a count and the clock, digested.
fn new_ticket() -> [u8; TICKET_BYTES] {
    use std::hash::{BuildHasher, Hasher};
    static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut hasher = blake3::Hasher::new();
    for _ in 0..2 {
        hasher.update(
            &std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish()
                .to_le_bytes(),
        );
    }
    hasher.update(
        &COUNT
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_le_bytes(),
    );
    hasher.update(
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
            .to_le_bytes(),
    );
    let mut ticket = [0u8; TICKET_BYTES];
    ticket.copy_from_slice(&hasher.finalize().as_bytes()[..TICKET_BYTES]);
    ticket
}

/// Keep a session's account, and give the ticket that asks for it — or
/// no ticket (an empty one) for an account that by itself holds more
/// files than this process keeps: the move that asked for it then removes
/// nothing, and says that it was given no account. A kept account answers
/// nothing until it is [`confirm`]ed.
pub fn keep(account: Account) -> Vec<u8> {
    let mut kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match keep_within(&mut kept, account, BOUNDS) {
        Some(ticket) => ticket.to_vec(),
        None => Vec::new(),
    }
}

/// [`keep`], among `kept` and within `bounds` (cr-uv4-5). An account
/// that is too large by itself is refused BEFORE anything is made room
/// for it: it used to push every older account out and then be kept
/// whole. Otherwise accounts kept too long go first, then the oldest
/// while there would be more files or more accounts than the bounds
/// allow.
fn keep_within(
    kept: &mut HashMap<[u8; TICKET_BYTES], Kept>,
    account: Account,
    bounds: Bounds,
) -> Option<[u8; TICKET_BYTES]> {
    if account.len() > bounds.files {
        log::warn!(
            "a move's account of {} files is more than this daemon keeps ({}): no ticket is \
             given for it, and the move will remove nothing",
            account.len(),
            bounds.files
        );
        return None;
    }
    kept.retain(|_, held| held.since.elapsed() < KEPT_FOR);
    let mut files: usize = kept.values().map(|held| held.state.len()).sum();
    while !kept.is_empty()
        && (files + account.len() > bounds.files || kept.len() >= bounds.accounts)
    {
        let oldest = *kept
            .iter()
            .min_by_key(|(_, held)| held.order)
            .map(|(ticket, _)| ticket)
            .expect("not empty");
        files -= kept.remove(&oldest).map_or(0, |held| held.state.len());
    }
    static ORDER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let ticket = new_ticket();
    kept.insert(
        ticket,
        Kept {
            state: State::Open(account),
            since: Instant::now(),
            order: ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        },
    );
    Some(ticket)
}

/// The session the account under `ticket` was written in has ended well,
/// and `not_landed` are the paths the receiving end says it does not hold
/// — each as the account was told it (the exact bytes of a name that is
/// not text). They are struck from the account, and from now on it
/// answers a removal. A ticket nothing answers to, or one confirmed
/// already, is left as it is.
pub fn confirm(ticket: &[u8], not_landed: &[Vec<u8>]) {
    let Ok(ticket) = <[u8; TICKET_BYTES]>::try_from(ticket) else {
        return;
    };
    let mut kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(held) = kept.get_mut(&ticket) else {
        return;
    };
    let State::Open(account) = &mut held.state else {
        return;
    };
    let struck: std::collections::HashSet<u128> =
        not_landed.iter().map(|path| path_digest(path)).collect();
    let landed = |pairs: Vec<(u128, u128)>| {
        let mut kept: Vec<u128> = pairs
            .into_iter()
            .filter(|(path, _)| !struck.contains(path))
            .map(|(_, whole)| whole)
            .collect();
        kept.sort_unstable();
        kept.dedup();
        kept
    };
    let sent = landed(std::mem::take(&mut account.sent));
    let contents = landed(std::mem::take(&mut account.contents));
    held.state = State::Confirmed(Arc::new(Closed {
        root: std::mem::take(&mut account.root),
        sent,
        contents,
    }));
}

/// The account under `ticket` is to answer nothing, ever: its session
/// ended without a whole word on what landed.
pub fn forget(ticket: &[u8]) {
    let Ok(ticket) = <[u8; TICKET_BYTES]>::try_from(ticket) else {
        return;
    };
    kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&ticket);
}

/// Whether these tickets are all known here and at least one of their
/// sessions has not ended yet: an answer may come, if it is waited for. A
/// ticket nothing answers to is never worth the wait.
pub fn pending(tickets: &[Vec<u8>]) -> bool {
    let kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut open = false;
    for ticket in tickets {
        let Ok(ticket) = <[u8; TICKET_BYTES]>::try_from(ticket.as_slice()) else {
            return false;
        };
        match kept.get(&ticket) {
            Some(held) => open |= matches!(held.state, State::Open(_)),
            None => return false,
        }
    }
    open
}

/// The accounts these tickets ask for: every one of them, confirmed, or
/// `None` when any is not held or its session has not ended well — never
/// some. They stay kept: a removal that was cut short may be asked again.
pub fn held(tickets: &[Vec<u8>]) -> Option<Vec<Arc<Closed>>> {
    let kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tickets
        .iter()
        .map(|ticket| {
            let ticket: [u8; TICKET_BYTES] = ticket.as_slice().try_into().ok()?;
            let held = kept.get(&ticket)?;
            match &held.state {
                State::Confirmed(account) if held.since.elapsed() < KEPT_FOR => {
                    Some(Arc::clone(account))
                }
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: i128 = 1_000_000_000;

    /// cr-uv7-1: an account holds a file's content by its path, and a
    /// path that did not land is struck from it with its version.
    #[test]
    fn an_account_holds_what_a_file_held_by_its_path() {
        let content = [7u8; 32];
        let other = [8u8; 32];
        let mut account = Account::of(Path::new("/srv/contents"));
        account.record(
            b"a",
            &Version {
                size: 1,
                mtime_nanos: 1,
                ctime_nanos: 1,
                device: 1,
                inode: 1,
            },
        );
        for name in [&b"a"[..], b"b", b"gone"] {
            account.record_content(name, &content);
        }
        let ticket = keep(account);
        confirm(&ticket, &[b"gone".to_vec()]);
        let accounts = held(&[ticket]).expect("kept");
        assert!(accounts[0].holds_content(b"a", &content));
        assert!(accounts[0].holds_content(b"b", &content));
        assert!(!accounts[0].holds_content(b"a", &other), "other content");
        assert!(!accounts[0].holds_content(b"c", &content), "another path");
        assert!(!accounts[0].holds_content(b"gone", &content), "struck");
    }

    /// cr-uv5-2: an account written from what a walk saw holds each file
    /// on the manifest as the walk saw it — changed in any way since, it
    /// is another file — and holds neither a file the walk saw that the
    /// manifest does not list, nor a name the walk never saw.
    #[test]
    fn an_account_is_written_from_what_the_walk_saw() {
        let seen = Version {
            size: 4,
            mtime_nanos: 1_700_000_000 * SECOND + 5,
            ctime_nanos: 1_700_000_000 * SECOND + 7,
            device: 1,
            inode: 9,
        };
        let sightings = Sightings::default();
        let root = Path::new("/srv/walked");
        assert!(
            sightings.account(root, &[name_key(b"a")]).is_empty(),
            "nothing seen, nothing held"
        );
        sightings.publish(vec![sighting(b"a", &seen), sighting(b"left-out", &seen)]);
        let account = sightings.account(root, &[name_key(b"a"), name_key(b"never-seen")]);
        assert_eq!(account.len(), 1, "what is listed AND was seen");
        let ticket = keep(account);
        confirm(&ticket, &[]);
        let accounts = held(&[ticket]).expect("kept");
        assert!(accounts[0].is_of(root));
        assert!(accounts[0].holds(b"a", &seen));
        // Rewritten since with as many bytes, its time put back: only the
        // time it was last changed says so.
        let rewritten = Version {
            ctime_nanos: seen.ctime_nanos + 1,
            ..seen
        };
        assert!(!accounts[0].holds(b"a", &rewritten));
        assert!(!accounts[0].holds(b"left-out", &seen));
        assert!(!accounts[0].holds(b"never-seen", &seen));
    }

    fn version(size: u64, mtime_nanos: i128) -> Version {
        Version {
            size,
            mtime_nanos,
            ctime_nanos: mtime_nanos + 5,
            device: 7,
            inode: 42,
        }
    }

    /// An account holds a file by its path and everything it was, and
    /// nothing it was not told of; a ticket asks for its own account, and
    /// tickets are answered all together or not at all.
    #[test]
    fn an_account_holds_what_was_sent_and_a_ticket_asks_for_it() {
        let at = 1_700_000_000 * SECOND + 120_000_000;
        let mut account = Account::of(Path::new("/srv/m/src"));
        account.record(b"a.txt", &version(5, at));
        account.record(b"sub/b\xff.bin", &version(0, -3));
        account.record(b"", &version(9, 1));
        account.record(b"a.txt", &version(5, at));
        let ticket = keep(account);
        assert_eq!(ticket.len(), TICKET_BYTES);
        confirm(&ticket, &[]);

        let accounts = held(std::slice::from_ref(&ticket)).expect("kept");
        let account = &accounts[0];
        assert_eq!(account.len(), 3, "a file told of twice is one");
        assert!(account.holds(b"a.txt", &version(5, at)));
        assert!(account.holds(b"sub/b\xff.bin", &version(0, -3)));
        assert!(account.holds(b"", &version(9, 1)));
        for (path, other) in [
            (&b"a.txt"[..], version(6, at)),
            (b"A.txt", version(5, at)),
            (b"a.tx", version(5, at)),
            (b"sub/b\xef\xbf\xbd.bin", version(0, -3)),
            (b"new.txt", version(5, at)),
        ] {
            assert!(!account.holds(path, &other), "{path:?} {other:?}");
        }

        // Another session's ticket is another account.
        let mut other = Account::of(Path::new("/srv/m/src"));
        other.record(b"new.txt", &version(5, at));
        let second = keep(other);
        confirm(&second, &[]);
        assert_ne!(ticket, second);
        let both = held(&[ticket.clone(), second.clone()]).expect("both kept");
        assert!(!both[0].holds(b"new.txt", &version(5, at)));
        assert!(both[1].holds(b"new.txt", &version(5, at)));

        // One ticket nothing answers to, and none of them is answered.
        assert!(held(&[ticket.clone(), vec![0u8; TICKET_BYTES]]).is_none());
        assert!(held(&[ticket[..8].to_vec()]).is_none());
        assert!(held(&[Vec::new()]).is_none());
        // No ticket asks for nothing, and gets it.
        assert!(held(&[]).is_some_and(|none| none.is_empty()));
        // A ticket is not used up.
        assert!(held(&[ticket]).is_some());
    }

    /// cr-uv4-1: a file is held as the file it was. The reviewer's case —
    /// other contents of the same length, written 800 ms later in the
    /// same second — is another file; so is one whose time was put back
    /// (its change time moved), and one put in its place (another file
    /// altogether, of the same size and times).
    #[test]
    fn a_file_rewritten_within_the_second_is_not_the_file_that_was_sent() {
        let sent = Version {
            size: 4096,
            mtime_nanos: 1_800_000_000 * SECOND + 100_000_000,
            ctime_nanos: 1_800_000_000 * SECOND + 100_000_000,
            device: 7,
            inode: 42,
        };
        let mut account = Account::of(Path::new("/srv/m/src"));
        account.record(b"f.bin", &sent);
        let ticket = keep(account);
        confirm(&ticket, &[]);
        let account = &held(&[ticket]).expect("kept")[0];
        assert!(account.holds(b"f.bin", &sent));

        let later = sent.mtime_nanos + 800_000_000;
        assert_eq!(later / SECOND, sent.mtime_nanos / SECOND, "the same second");
        let rewritten = Version {
            mtime_nanos: later,
            ctime_nanos: later,
            ..sent
        };
        assert!(!account.holds(b"f.bin", &rewritten));
        // Each of the two times tells by itself.
        let written_later = Version {
            mtime_nanos: later,
            ..sent
        };
        assert!(!account.holds(b"f.bin", &written_later));
        // Written to, and its time put back: the change time tells.
        let put_back = Version {
            ctime_nanos: later,
            ..sent
        };
        assert!(!account.holds(b"f.bin", &put_back));
        // Another file under the name, with the size and both times.
        let replaced = Version { inode: 43, ..sent };
        assert!(!account.holds(b"f.bin", &replaced));
        let elsewhere = Version { device: 8, ..sent };
        assert!(!account.holds(b"f.bin", &elsewhere));
    }

    /// cr-uv4-1: an account answers nothing until its session has ended
    /// well — the ticket is given before a byte of any file has gone —
    /// and then holds nothing the receiving end said it does not have.
    /// One whose session never ends well is never answered.
    #[test]
    fn an_account_answers_only_once_its_session_has_ended_well() {
        let at = 1_700_000_000 * SECOND;
        let mut account = Account::of(Path::new("/srv/m/src"));
        for path in [
            &b"landed.txt"[..],
            b"failed.txt",
            b"sub/bad\xff.bin",
            b"kept",
        ] {
            account.record(path, &version(1, at));
        }
        let ticket = keep(account);
        assert!(
            held(std::slice::from_ref(&ticket)).is_none(),
            "the session has not ended"
        );
        // It may yet: worth a wait. A ticket nothing answers to is not.
        assert!(pending(std::slice::from_ref(&ticket)));
        assert!(!pending(&[ticket.clone(), vec![0u8; TICKET_BYTES]]));
        assert!(!pending(&[ticket[..4].to_vec()]));
        assert!(!pending(&[]));
        // A word about another ticket confirms nothing here.
        confirm(&[0u8; TICKET_BYTES], &[]);
        confirm(&ticket[..4], &[]);
        assert!(held(std::slice::from_ref(&ticket)).is_none());

        confirm(
            &ticket,
            &[
                b"failed.txt".to_vec(),
                b"sub/bad\xff.bin".to_vec(),
                b"never sent".to_vec(),
            ],
        );
        assert!(!pending(std::slice::from_ref(&ticket)), "it has ended");
        let accounts = held(std::slice::from_ref(&ticket)).expect("confirmed");
        let account = &accounts[0];
        assert_eq!(account.len(), 2);
        assert!(account.holds(b"landed.txt", &version(1, at)));
        assert!(account.holds(b"kept", &version(1, at)));
        assert!(!account.holds(b"failed.txt", &version(1, at)));
        assert!(!account.holds(b"sub/bad\xff.bin", &version(1, at)));
        // The text a name that is not text is shown as is another path.
        assert!(!account.holds(b"sub/bad\xef\xbf\xbd.bin", &version(1, at)));
        // Confirmed once: a second word changes nothing.
        confirm(&ticket, &[b"landed.txt".to_vec()]);
        assert!(
            held(std::slice::from_ref(&ticket)).unwrap()[0].holds(b"landed.txt", &version(1, at))
        );

        // A session that ended without a whole word on what landed.
        let mut lost = Account::of(Path::new("/srv/m/src"));
        lost.record(b"x", &version(1, at));
        let lost = keep(lost);
        forget(&lost);
        confirm(&lost, &[]);
        assert!(held(&[lost]).is_none());
    }

    /// cr-uv4-1: an account is of one place — the path its session read —
    /// however that path is spelled, and of no other.
    #[test]
    fn an_account_is_of_the_place_its_session_read() {
        let mut account = Account::of(Path::new("/srv/m/src/"));
        account.record(b"a", &version(1, 1));
        let ticket = keep(account);
        confirm(&ticket, &[]);
        let account = &held(&[ticket]).unwrap()[0];
        for same in ["/srv/m/src", "/srv/m/src/", "/srv/m/./src/."] {
            assert!(account.is_of(Path::new(same)), "{same}");
        }
        for other in ["/srv/m/src2", "/srv/m", "/srv/m/src/sub", "/srv/other/src"] {
            assert!(!account.is_of(Path::new(other)), "{other}");
        }
    }

    /// What a file's metadata gives: its size and its time to the part of
    /// a second the filesystem keeps; a write is another version.
    #[test]
    fn a_version_is_read_from_the_file_whole() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let file = tmp.path().join("f");
        std::fs::write(&file, b"1234").unwrap();
        let at = filetime::FileTime::from_unix_time(1_800_000_000, 100_000_000);
        filetime::set_file_mtime(&file, at).unwrap();
        let first = Version::of(&std::fs::metadata(&file).unwrap());
        assert_eq!(first.size, 4);
        assert_eq!(first.mtime_nanos / SECOND, 1_800_000_000);
        // Other contents of the same length, 800 ms later in that second.
        std::fs::write(&file, b"abcd").unwrap();
        let later = filetime::FileTime::from_unix_time(1_800_000_000, 900_000_000);
        filetime::set_file_mtime(&file, later).unwrap();
        let second = Version::of(&std::fs::metadata(&file).unwrap());
        assert_eq!(second.size, first.size);
        assert_eq!(second.mtime_nanos / SECOND, first.mtime_nanos / SECOND);
        if first.mtime_nanos % SECOND != 0 {
            // (A filesystem that keeps whole seconds cannot show this.)
            assert_ne!(second, first);
            assert_eq!(second.mtime_nanos - first.mtime_nanos, 800_000_000);
        }
        #[cfg(unix)]
        {
            // And its time put back, the change time has still moved.
            std::thread::sleep(Duration::from_millis(20));
            filetime::set_file_mtime(&file, at).unwrap();
            let put_back = Version::of(&std::fs::metadata(&file).unwrap());
            assert_eq!(put_back.mtime_nanos, first.mtime_nanos);
            assert_eq!(put_back.inode, first.inode);
            assert_ne!(put_back.ctime_nanos, first.ctime_nanos);
        }
    }

    /// cr-uv4-5: what is kept of moves stays within its bounds. An
    /// account that by itself holds more files than may be kept gets no
    /// ticket and pushes nothing out — it used to push every older
    /// account out and be kept whole. Otherwise the oldest accounts make
    /// room, for the files and for the number of accounts alike, and a
    /// session still open counts like any other.
    #[test]
    fn what_is_kept_of_moves_stays_within_its_bounds() {
        let bounds = Bounds {
            files: 10,
            accounts: 3,
        };
        let of = |files: usize| {
            let mut account = Account::of(Path::new("/srv/m/src"));
            for at in 0..files {
                account.record(format!("f{at}").as_bytes(), &version(1, at as i128));
            }
            account
        };
        let mut kept: HashMap<[u8; TICKET_BYTES], Kept> = HashMap::new();
        let first = keep_within(&mut kept, of(4), bounds).expect("kept");
        let second = keep_within(&mut kept, of(4), bounds).expect("kept");
        // One account over the bound by itself: no ticket, and both
        // older accounts are still there.
        assert!(keep_within(&mut kept, of(11), bounds).is_none());
        assert_eq!(kept.len(), 2);
        assert!(kept.contains_key(&first) && kept.contains_key(&second));
        // One that fits only without the oldest: the oldest goes.
        let third = keep_within(&mut kept, of(5), bounds).expect("kept");
        assert!(!kept.contains_key(&first), "the oldest made room");
        assert!(kept.contains_key(&second) && kept.contains_key(&third));
        let held: usize = kept.values().map(|held| held.state.len()).sum();
        assert_eq!(held, 9);
        // One the size of the bound itself is kept, alone.
        let whole = keep_within(&mut kept, of(10), bounds).expect("kept");
        assert_eq!(kept.len(), 1);
        assert!(kept.contains_key(&whole));

        // The number of accounts is bounded too, whatever they hold: a
        // session that sent nothing has a ticket all the same.
        let mut kept: HashMap<[u8; TICKET_BYTES], Kept> = HashMap::new();
        let tickets: Vec<_> = (0..5)
            .map(|_| keep_within(&mut kept, of(0), bounds).expect("kept"))
            .collect();
        assert_eq!(kept.len(), 3);
        for (at, ticket) in tickets.iter().enumerate() {
            assert_eq!(kept.contains_key(ticket), at >= 2, "the {at}th");
        }
    }
}

/// cr-uv8-1: what a file holds is known only of the file as it was seen.
#[cfg(all(test, unix))]
mod content_tests {
    use super::*;
    use std::os::unix::fs::{FileExt as _, MetadataExt as _};

    /// A hook on each read of what the file at `path` holds, while the
    /// guard lives.
    struct Hooked((u64, u64));

    impl Drop for Hooked {
        fn drop(&mut self) {
            if let Ok(mut hooks) = WHILE_CONTENT_IS_READ_HOOKS.lock() {
                hooks.retain(|(file, _)| *file != self.0);
            }
        }
    }

    fn hook(path: &Path, hook: impl Fn(&'static str, u64) + Send + Sync + 'static) -> Hooked {
        let metadata = std::fs::metadata(path).unwrap();
        let file = (metadata.dev(), metadata.ino());
        WHILE_CONTENT_IS_READ_HOOKS
            .lock()
            .unwrap()
            .push((file, Arc::new(hook)));
        Hooked(file)
    }

    fn seen(path: &Path) -> Version {
        Version::of(&std::fs::symlink_metadata(path).unwrap())
    }

    fn mtime_of(path: &Path) -> filetime::FileTime {
        filetime::FileTime::from_last_modification_time(&std::fs::metadata(path).unwrap())
    }

    /// Unchanged, its digest. Written to once read — its first bytes,
    /// which the digest has, as many as there were, its time put back, so
    /// that no look but at the time it was last changed sees it — or
    /// another file of the same bytes and time: nothing is known of what
    /// it holds.
    #[test]
    fn a_content_is_known_only_of_the_file_as_it_was_seen() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("one");
        std::fs::write(&path, b"one file").unwrap();
        let version = seen(&path);
        assert_eq!(
            content_of(&std::fs::File::open(&path).unwrap(), &version).unwrap(),
            *blake3::hash(b"one file").as_bytes()
        );
        {
            let target = path.clone();
            let _hooked = hook(&path, move |stage, _| {
                if stage == "read" {
                    let was = mtime_of(&target);
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .open(&target)
                        .unwrap();
                    file.write_at(b"two", 0).unwrap();
                    filetime::set_file_mtime(&target, was).unwrap();
                }
            });
            assert!(content_of(&std::fs::File::open(&path).unwrap(), &version).is_err());
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"two file", "it was written");

        let version = seen(&path);
        let other = tmp.path().join("other");
        std::fs::write(&other, b"two file").unwrap();
        filetime::set_file_mtime(&other, mtime_of(&path)).unwrap();
        assert_eq!(seen(&other).mtime_nanos, version.mtime_nanos);
        assert!(content_of(&std::fs::File::open(&other).unwrap(), &version).is_err());
        assert!(content_of(&std::fs::File::open(&path).unwrap(), &version).is_ok());
    }

    /// cr-uv9-2: a descriptor that is not the file as it was seen — another
    /// file, of the same bytes and time, or this one changed — is not read
    /// at all: nothing of it is consumed.
    #[test]
    fn a_file_that_is_not_the_one_is_not_read() {
        use std::io::Seek as _;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("one");
        std::fs::write(&path, vec![1u8; 70_000]).unwrap();
        let version = seen(&path);
        let other = tmp.path().join("other");
        std::fs::write(&other, vec![1u8; 70_000]).unwrap();
        filetime::set_file_mtime(&other, mtime_of(&path)).unwrap();
        let mut file = std::fs::File::open(&other).unwrap();
        assert!(content_of(&file, &version).is_err());
        assert_eq!(file.stream_position().unwrap(), 0, "another file");

        let was = mtime_of(&path);
        std::fs::write(&path, vec![2u8; 70_000]).unwrap();
        filetime::set_file_mtime(&path, was).unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        assert!(content_of(&file, &version).is_err());
        assert_eq!(file.stream_position().unwrap(), 0, "the file changed");
    }

    /// No further than the size it was seen with: a file that grows as it
    /// is read is not read on, and is not the file that was seen.
    #[test]
    fn a_content_read_goes_no_further_than_the_size_seen() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("growing");
        std::fs::write(&path, b"one file").unwrap();
        let version = seen(&path);
        let read = Arc::new(Mutex::new(None));
        {
            let (target, read) = (path.clone(), Arc::clone(&read));
            let _hooked = hook(&path, move |stage, so_far| match stage {
                "reading" => {
                    use std::io::Write as _;
                    std::fs::OpenOptions::new()
                        .append(true)
                        .open(&target)
                        .unwrap()
                        .write_all(&[7u8; 100_000])
                        .unwrap();
                }
                "read" => *read.lock().unwrap() = Some(so_far),
                _ => {}
            });
            assert!(content_of(&std::fs::File::open(&path).unwrap(), &version).is_err());
        }
        assert_eq!(*read.lock().unwrap(), Some(version.size));
    }
}
