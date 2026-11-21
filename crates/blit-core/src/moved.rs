//! What a move's session sent, kept by the end that sent it (cr-uv3-1,
//! `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`).
//!
//! A move from a daemon copies, and then asks the daemon to remove the
//! source. That removal was a recursive `rm` of whatever stood at the
//! source by then — so a link, which a transfer does not carry, went with
//! the files that were copied, and so would a file that arrived after the
//! source had been read. Nothing tied the removal to the transfer.
//!
//! The end that is read from now keeps an ACCOUNT of every file a session
//! put on its manifest, when the session was opened as one whose source
//! will be removed, and hands the other end a TICKET for it. The removal
//! presents the tickets of the move's sessions, and takes a file only if
//! one of those accounts holds it — by its path, its size and its
//! modification time, as the manifest wrote them. Whatever no account
//! holds is left where it is.
//!
//! An account is a sorted list of 128-bit digests, sixteen bytes a file,
//! held in this process: a daemon that restarts forgets them, and the
//! removal is then refused whole, with the source as it was.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// A ticket is this many bytes.
pub const TICKET_BYTES: usize = 16;

/// How long an account is kept: a move's retries, with their waits, end
/// well inside it.
const KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// The most files all kept accounts hold between them — a gibibyte of
/// digests. Past it the oldest account goes, and a removal that presents
/// its ticket is refused.
const MOST_FILES_KEPT: usize = 64 * 1024 * 1024;

fn digest(path: &[u8], size: u64, mtime_seconds: i64) -> u128 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(path.len() as u64).to_le_bytes());
    hasher.update(path);
    hasher.update(&size.to_le_bytes());
    hasher.update(&mtime_seconds.to_le_bytes());
    let mut first = [0u8; 16];
    first.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    u128::from_le_bytes(first)
}

/// One session's account, as it is written.
#[derive(Debug, Default)]
pub struct Account {
    sent: Vec<u128>,
}

impl Account {
    /// A file the session put on its manifest: its path below the source
    /// (`/` between names, the exact bytes of a name that is not text;
    /// empty for a source that is itself the file), and the size and
    /// modification time the manifest gave it.
    pub fn record(&mut self, path: &[u8], size: u64, mtime_seconds: i64) {
        self.sent.push(digest(path, size, mtime_seconds));
    }

    /// The account, closed: nothing more is written to it.
    pub fn close(mut self) -> Closed {
        self.sent.sort_unstable();
        self.sent.dedup();
        Closed { sent: self.sent }
    }
}

/// A session's account, closed.
#[derive(Debug)]
pub struct Closed {
    sent: Vec<u128>,
}

impl Closed {
    /// Whether the session sent this file: this path, at this size and
    /// time. A file written to since, or put there since, is another.
    pub fn holds(&self, path: &[u8], size: u64, mtime_seconds: i64) -> bool {
        self.sent
            .binary_search(&digest(path, size, mtime_seconds))
            .is_ok()
    }

    pub fn len(&self) -> usize {
        self.sent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sent.is_empty()
    }
}

struct Kept {
    account: Arc<Closed>,
    since: Instant,
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

/// Keep a session's account, and give the ticket that asks for it.
/// Accounts kept too long go first, then the oldest while there are more
/// files held than [`MOST_FILES_KEPT`].
pub fn keep(account: Account) -> Vec<u8> {
    let account = Arc::new(account.close());
    let mut kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    kept.retain(|_, held| held.since.elapsed() < KEPT_FOR);
    let mut files: usize = kept.values().map(|held| held.account.len()).sum();
    while files + account.len() > MOST_FILES_KEPT && !kept.is_empty() {
        let oldest = *kept
            .iter()
            .min_by_key(|(_, held)| held.since)
            .map(|(ticket, _)| ticket)
            .expect("not empty");
        files -= kept.remove(&oldest).map_or(0, |held| held.account.len());
    }
    let ticket = new_ticket();
    kept.insert(
        ticket,
        Kept {
            account,
            since: Instant::now(),
        },
    );
    ticket.to_vec()
}

/// The accounts these tickets ask for: every one of them, or `None` when
/// any is not held — never some. They stay kept: a removal that was cut
/// short may be asked again.
pub fn held(tickets: &[Vec<u8>]) -> Option<Vec<Arc<Closed>>> {
    let kept = kept()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tickets
        .iter()
        .map(|ticket| {
            let ticket: [u8; TICKET_BYTES] = ticket.as_slice().try_into().ok()?;
            let held = kept.get(&ticket)?;
            (held.since.elapsed() < KEPT_FOR).then(|| Arc::clone(&held.account))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An account holds a file by its path, size and time together, and
    /// nothing it was not told of; a ticket asks for its own account, and
    /// tickets are answered all together or not at all.
    #[test]
    fn an_account_holds_what_was_sent_and_a_ticket_asks_for_it() {
        let mut account = Account::default();
        account.record(b"a.txt", 5, 1_700_000_000);
        account.record(b"sub/b\xff.bin", 0, -3);
        account.record(b"", 9, 1);
        account.record(b"a.txt", 5, 1_700_000_000);
        let ticket = keep(account);
        assert_eq!(ticket.len(), TICKET_BYTES);

        let accounts = held(std::slice::from_ref(&ticket)).expect("kept");
        let account = &accounts[0];
        assert_eq!(account.len(), 3, "a file told of twice is one");
        assert!(account.holds(b"a.txt", 5, 1_700_000_000));
        assert!(account.holds(b"sub/b\xff.bin", 0, -3));
        assert!(account.holds(b"", 9, 1));
        for (path, size, mtime) in [
            (&b"a.txt"[..], 6, 1_700_000_000),
            (b"a.txt", 5, 1_700_000_001),
            (b"A.txt", 5, 1_700_000_000),
            (b"a.tx", 5, 1_700_000_000),
            (b"sub/b\xef\xbf\xbd.bin", 0, -3),
            (b"new.txt", 5, 1_700_000_000),
        ] {
            assert!(!account.holds(path, size, mtime), "{path:?} {size} {mtime}");
        }

        // Another session's ticket is another account.
        let mut other = Account::default();
        other.record(b"new.txt", 5, 1_700_000_000);
        let second = keep(other);
        assert_ne!(ticket, second);
        let both = held(&[ticket.clone(), second.clone()]).expect("both kept");
        assert!(!both[0].holds(b"new.txt", 5, 1_700_000_000));
        assert!(both[1].holds(b"new.txt", 5, 1_700_000_000));

        // One ticket nothing answers to, and none of them is answered.
        assert!(held(&[ticket.clone(), vec![0u8; TICKET_BYTES]]).is_none());
        assert!(held(&[ticket[..8].to_vec()]).is_none());
        assert!(held(&[Vec::new()]).is_none());
        // No ticket asks for nothing, and gets it.
        assert!(held(&[]).is_some_and(|none| none.is_empty()));
        // A ticket is not used up.
        assert!(held(&[ticket]).is_some());
    }
}
