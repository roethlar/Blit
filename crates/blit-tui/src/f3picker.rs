//! F3 picker mode — the single-shot "pick a path" invocation of the
//! F3 browse pane (`docs/plan/TUI_REWORK.md` §4.2, milestone M1).
//!
//! The rework's driving principle is "pick, don't type": every path a
//! transfer needs should be chosen off a navigable list, never recalled
//! from memory and typed. F3 already owns the navigation machinery
//! (modules, directories, filter, cursor, marks), so the rework promotes
//! that one browser into a reusable picker instead of growing a second
//! one. This module is the state half of that promotion: it records
//! whether a pick is in flight, what kind of location the invoker asked
//! for, and where to send the answer. Navigation itself stays in
//! [`crate::browse::BrowseState`]; keystroke routing lives in
//! `main::handle_f3_picker_keystroke`.
//!
//! Keymap (§4.2):
//!
//! - `Enter` — **always descends** into the highlighted entry, never
//!   "confirms". On a *file* row it is terminal, but only in a picker
//!   that accepts files; in a directory picker it is inert.
//! - `.` — picks the directory currently being **viewed** (not the
//!   cursor row); inert in a file picker.
//! - `Esc` — cancels. The continuation is dropped, so the invoker's
//!   `await` resolves to `Err(RecvError)` and takes its cancel path.
//! - pane switching — disabled while picking, since switching panes
//!   mid-pick would orphan the continuation.
//!
//! Enter-always-descends is the round-1 review fix: a directory is
//! chosen by navigating INTO it and pressing `.`, never by pointing at
//! its row, so `Enter` never means two things depending on where the
//! cursor happens to sit.
//!
//! Continuation hygiene (§7): the answer rides a
//! `tokio::sync::oneshot::Sender<PathPicked>` owned by the `Picking`
//! state. Every termination path drops that sender — a pick, `Esc`, a
//! second invocation, a quit that drops `AppState`, or an unwind through
//! it — so an invoker can never wait forever on a picker that is no
//! longer on screen.
//!
//! **M1 scope**: the mode and the path-return plumbing only. No
//! keystroke opens a picker yet and no existing behavior changes —
//! [`F3PickerState::begin`] is the entry point M3a (F1 trigger modal)
//! and M3b (F3 pull destination) call. Two §4.2 pieces deliberately land
//! with those callers because neither has an answer yet: the start
//! location (§6 decision 2 is unratified, and there is no browsable
//! local root until M2's `LocalDaemon`), and the restore-the-prior-pane
//! half of §7's transient-state reset (M1 has no prior pane to return
//! to).

use blit_core::endpoints::Endpoint;
use tokio::sync::oneshot;

/// What the invoking flow asked the picker to return (§4.2's "what kind
/// of picker" hint — file? directory? either?).
///
/// The kind is the whole difference between the two terminal keys: it
/// decides which of `Enter`-on-a-file and `.` is live and which is a
/// documented no-op. A source pick is usually [`PickerKind::Either`] (a
/// transfer source may be one file or a whole tree); a destination pick
/// is usually [`PickerKind::Directory`].
///
/// M1 has no caller yet, so nothing in the binary constructs a variant —
/// M3a/M3b do, and the allow goes away with them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    /// Only a file may be returned. `Enter` on a file row picks it; `.`
    /// is inert.
    File,
    /// Only a directory may be returned. `.` picks the directory being
    /// viewed; `Enter` on a file row is inert.
    Directory,
    /// Either shape may be returned — both terminal keys are live.
    Either,
}

impl PickerKind {
    /// May `Enter` on a file row end this pick?
    pub fn allows_file(self) -> bool {
        matches!(self, Self::File | Self::Either)
    }

    /// May `.` end this pick with the directory being viewed?
    pub fn allows_directory(self) -> bool {
        matches!(self, Self::Directory | Self::Either)
    }

    /// §4.2's header title suffix for this kind.
    fn title_suffix(self) -> &'static str {
        match self {
            Self::File => " · picker (file)",
            Self::Directory => " · picker (directory)",
            Self::Either => " · picker (file or directory)",
        }
    }
}

/// Which shape the operator actually landed on. The invoker needs this
/// to know whether it was handed a container or a leaf without
/// re-statting the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickTarget {
    File,
    Directory,
}

/// What the picker hands back through the continuation (§4.2's
/// `PathPicked`).
///
/// The location is a `blit_app` [`Endpoint`] rather than a bare
/// `RemoteEndpoint` so M2's local browsing needs no second shape and
/// `plan_f1_trigger` can consume the value directly (§4.3).
///
/// M1 wires no reader, so the fields are written and never read inside
/// the binary — M3a/M3b read them.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PathPicked {
    /// The location the operator chose.
    pub endpoint: Endpoint,
    /// Whether that location is a file or a directory.
    pub target: PickTarget,
}

/// Lifecycle of the F3 picker.
#[derive(Debug)]
enum F3PickerStatus {
    /// F3 is in its normal Browse mode — no pick in flight.
    Idle,
    /// A pick is in flight. Holding the sender here is what makes the
    /// §7 hygiene automatic: whatever ends this state drops it.
    Picking {
        kind: PickerKind,
        reply: oneshot::Sender<PathPicked>,
    },
}

/// The F3 browse pane's picker-mode flag plus its return channel.
#[derive(Debug)]
pub struct F3PickerState {
    status: F3PickerStatus,
}

impl Default for F3PickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl F3PickerState {
    pub fn new() -> Self {
        Self {
            status: F3PickerStatus::Idle,
        }
    }

    /// `true` while a pick is in flight — the input router consults this
    /// to route F3 keystrokes through the picker keymap instead of the
    /// normal Browse dispatcher.
    pub fn is_picking(&self) -> bool {
        matches!(self.status, F3PickerStatus::Picking { .. })
    }

    /// The kind of pick in flight, or `None` while Idle.
    ///
    /// Test-only: the §4.2 file-vs-directory rule is enforced inside
    /// [`F3PickerState::pick`], so no production caller has to ask, and
    /// an accessor nobody calls is one more thing to keep honest.
    #[cfg(test)]
    pub fn kind(&self) -> Option<PickerKind> {
        match &self.status {
            F3PickerStatus::Picking { kind, .. } => Some(*kind),
            F3PickerStatus::Idle => None,
        }
    }

    /// Open picker mode for `kind` and hand the invoker the receiver it
    /// awaits. `Ok(picked)` is a pick; `Err(RecvError)` is every form of
    /// cancel, so a caller only ever needs two arms.
    ///
    /// A second invocation while a pick is already in flight does NOT
    /// disturb the running one: the new sender is dropped immediately,
    /// so the second caller's await resolves to `Err(RecvError)` (its
    /// cancel path) rather than hanging on a picker that will answer
    /// someone else. The picker is modal, so this is defensive.
    ///
    /// M1 wires no caller — M3a/M3b are the callers, and the allow goes
    /// away with them.
    #[allow(dead_code)]
    #[must_use = "the receiver is the only way to learn what was picked"]
    pub fn begin(&mut self, kind: PickerKind) -> oneshot::Receiver<PathPicked> {
        let (reply, rx) = oneshot::channel();
        if self.is_picking() {
            // `reply` drops here — the newcomer is cancelled, the
            // in-flight pick is untouched.
            return rx;
        }
        self.status = F3PickerStatus::Picking { kind, reply };
        rx
    }

    /// `Enter` landed on a file row: return it and leave picker mode.
    ///
    /// Returns `false` — leaving the picker exactly as it was — when no
    /// pick is in flight or when the invoker asked for a directory. That
    /// `false` is §4.2's "Enter on a file in directory-picker mode is a
    /// no-op".
    pub fn pick_file(&mut self, endpoint: Endpoint) -> bool {
        self.pick(endpoint, PickTarget::File)
    }

    /// `.` on the directory being viewed: return it and leave picker
    /// mode. Returns `false` (picker untouched) when no pick is in
    /// flight or when the invoker asked for a file.
    pub fn pick_directory(&mut self, endpoint: Endpoint) -> bool {
        self.pick(endpoint, PickTarget::Directory)
    }

    /// Shared body of the two terminal keys. A pick that its invoker is
    /// no longer listening for still closes picker mode — the mode
    /// tracks what is on screen, not what anyone is waiting for.
    fn pick(&mut self, endpoint: Endpoint, target: PickTarget) -> bool {
        let allowed = match (&self.status, target) {
            (F3PickerStatus::Picking { kind, .. }, PickTarget::File) => kind.allows_file(),
            (F3PickerStatus::Picking { kind, .. }, PickTarget::Directory) => {
                kind.allows_directory()
            }
            (F3PickerStatus::Idle, _) => false,
        };
        if !allowed {
            return false;
        }
        if let F3PickerStatus::Picking { reply, .. } =
            std::mem::replace(&mut self.status, F3PickerStatus::Idle)
        {
            let _ = reply.send(PathPicked { endpoint, target });
        }
        true
    }

    /// `Esc` (and every other abandonment): leave picker mode and drop
    /// the continuation, so the invoker's await resolves to
    /// `Err(RecvError)` and it takes its cancel path (§7). No-op while
    /// Idle.
    pub fn cancel(&mut self) {
        self.status = F3PickerStatus::Idle;
    }

    /// §4.2's visual signal, in its minimal form: the suffix the F3
    /// header appends while picking. Empty while Idle, so the header
    /// renders byte-for-byte as it did before M1.
    ///
    /// §6 decision 6 (accent border + title suffix + status-bar hint) is
    /// NOT owner-ratified, so only this one non-contentious piece ships
    /// — one `&'static str`, trivially retuned or dropped when the owner
    /// rules.
    pub fn title_suffix(&self) -> &'static str {
        match &self.status {
            F3PickerStatus::Picking { kind, .. } => kind.title_suffix(),
            F3PickerStatus::Idle => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::remote::endpoint::RemoteEndpoint;
    use tokio::sync::oneshot::error::TryRecvError;

    fn remote(raw: &str) -> Endpoint {
        Endpoint::Remote(RemoteEndpoint::parse(raw).expect("endpoint"))
    }

    fn spec(endpoint: &Endpoint) -> String {
        match endpoint {
            Endpoint::Remote(ep) => ep.display(),
            Endpoint::Local(path) => path.display().to_string(),
        }
    }

    #[test]
    fn new_is_idle() {
        let s = F3PickerState::new();
        assert!(!s.is_picking());
        assert_eq!(s.kind(), None);
        assert_eq!(s.title_suffix(), "", "an idle F3 header gains nothing");
    }

    #[test]
    fn begin_opens_picker_mode_for_the_requested_kind() {
        let mut s = F3PickerState::new();
        let _rx = s.begin(PickerKind::Directory);
        assert!(s.is_picking());
        assert_eq!(s.kind(), Some(PickerKind::Directory));
    }

    /// §4.2: `Enter` on a file is the file picker's terminal key — the
    /// path reaches the invoker and the picker closes.
    #[tokio::test]
    async fn a_file_pick_returns_the_path_to_the_invoker() {
        let mut s = F3PickerState::new();
        let rx = s.begin(PickerKind::File);
        assert!(s.pick_file(remote("nas:/photos/2024/img.raw")));
        assert!(!s.is_picking(), "a pick ends picker mode");
        let picked = rx.await.expect("the continuation carries the pick");
        assert_eq!(spec(&picked.endpoint), "nas:/photos/2024/img.raw");
        assert_eq!(picked.target, PickTarget::File);
    }

    /// §4.2: `.` is the directory picker's terminal key.
    #[tokio::test]
    async fn a_directory_pick_returns_the_viewed_directory() {
        let mut s = F3PickerState::new();
        let rx = s.begin(PickerKind::Directory);
        assert!(s.pick_directory(remote("nas:/photos/2024")));
        assert!(!s.is_picking());
        let picked = rx.await.expect("the continuation carries the pick");
        assert_eq!(spec(&picked.endpoint), "nas:/photos/2024");
        assert_eq!(picked.target, PickTarget::Directory);
    }

    /// §4.2: in a DIRECTORY picker, `Enter` on a file is a no-op — the
    /// invoker asked for a container and must not be handed a leaf.
    #[test]
    fn a_directory_picker_refuses_a_file_pick() {
        let mut s = F3PickerState::new();
        let mut rx = s.begin(PickerKind::Directory);
        assert!(!s.pick_file(remote("nas:/photos/2024/img.raw")));
        assert!(s.is_picking(), "the refused pick leaves the picker open");
        assert_eq!(s.kind(), Some(PickerKind::Directory));
        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "nothing was sent to the invoker"
        );
    }

    /// The mirror rule: a FILE picker's `.` is inert.
    #[test]
    fn a_file_picker_refuses_a_directory_pick() {
        let mut s = F3PickerState::new();
        let mut rx = s.begin(PickerKind::File);
        assert!(!s.pick_directory(remote("nas:/photos/2024")));
        assert!(s.is_picking());
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    /// §4.2's third hint shape: an "either" picker has both terminal
    /// keys live.
    #[tokio::test]
    async fn an_either_picker_accepts_both_shapes() {
        let mut s = F3PickerState::new();
        let rx = s.begin(PickerKind::Either);
        assert!(s.pick_file(remote("nas:/docs/readme.txt")));
        assert_eq!(rx.await.expect("file pick").target, PickTarget::File);

        let rx = s.begin(PickerKind::Either);
        assert!(s.pick_directory(remote("nas:/docs/")));
        assert_eq!(rx.await.expect("dir pick").target, PickTarget::Directory);
    }

    /// §7: `Esc` drops the continuation, so the invoker's await resolves
    /// to `Err(RecvError)` and it falls back to its cancel path — it
    /// never waits on a picker that is no longer on screen.
    #[tokio::test]
    async fn cancel_drops_the_continuation() {
        let mut s = F3PickerState::new();
        let rx = s.begin(PickerKind::Directory);
        s.cancel();
        assert!(!s.is_picking());
        assert!(rx.await.is_err(), "cancel reaches the invoker as an error");
    }

    #[test]
    fn cancel_while_idle_is_a_no_op() {
        let mut s = F3PickerState::new();
        s.cancel();
        assert!(!s.is_picking());
    }

    /// Defensive (the picker is modal): a second invocation cancels
    /// ITSELF rather than orphaning the pick already in flight.
    #[test]
    fn a_second_invocation_never_orphans_the_first() {
        let mut s = F3PickerState::new();
        let mut first = s.begin(PickerKind::File);
        let mut second = s.begin(PickerKind::Directory);
        assert_eq!(
            s.kind(),
            Some(PickerKind::File),
            "the in-flight pick keeps the pane"
        );
        assert!(
            matches!(second.try_recv(), Err(TryRecvError::Closed)),
            "the newcomer is cancelled immediately"
        );
        assert!(
            matches!(first.try_recv(), Err(TryRecvError::Empty)),
            "the original invoker is still waiting, not cancelled"
        );
    }

    /// An invoker that gave up (dropped its receiver) does not strand
    /// the pane in picker mode.
    #[test]
    fn a_pick_with_no_listener_still_closes_picker_mode() {
        let mut s = F3PickerState::new();
        drop(s.begin(PickerKind::File));
        assert!(s.pick_file(remote("nas:/docs/readme.txt")));
        assert!(!s.is_picking());
    }

    #[test]
    fn kind_gates_match_the_spec_table() {
        assert!(PickerKind::File.allows_file());
        assert!(!PickerKind::File.allows_directory());
        assert!(PickerKind::Directory.allows_directory());
        assert!(!PickerKind::Directory.allows_file());
        assert!(PickerKind::Either.allows_file());
        assert!(PickerKind::Either.allows_directory());
    }

    /// §4.2's title suffix names the kind the caller asked for, so the
    /// operator can see which key ends the pick.
    #[test]
    fn title_suffix_names_the_picker_kind() {
        let mut s = F3PickerState::new();
        let _rx = s.begin(PickerKind::File);
        assert_eq!(s.title_suffix(), " · picker (file)");
        s.cancel();
        let _rx = s.begin(PickerKind::Directory);
        assert_eq!(s.title_suffix(), " · picker (directory)");
        s.cancel();
        let _rx = s.begin(PickerKind::Either);
        assert_eq!(s.title_suffix(), " · picker (file or directory)");
        s.cancel();
        assert_eq!(s.title_suffix(), "");
    }
}
