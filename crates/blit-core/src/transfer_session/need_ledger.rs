//! The DESTINATION's one ledger of granted needs (contract v7,
//! `docs/plan/SOURCE_SIDE_CONTAINMENT.md` D-A, ssc-1).
//!
//! Before v7 the destination tracked a granted need across three shared
//! sets — the outstanding completion set, the retained manifest headers,
//! and the resume-grant map — all removed at claim time, so a record's
//! terminator had nothing left to look up. This ledger keeps every
//! granted path for the whole session and moves it through explicit
//! states:
//!
//! ```text
//! grant ─▶ Granted ─┬─ skip ─────────────────────────▶ Failed
//!                   ├─ FileBegin / FILE tag ─▶ Active(lane) ─┬─ end ok ─▶ Completed
//!                   │                                        └─ end failed ▶ Failed
//!                   ├─ shard member ──────────────▶ Completed | Failed (per member)
//!                   └─ first block ──▶ Active(lane, Resume) ─┬─ complete ok ▶ Completed
//!                   │                                        └─ complete failed ▶ Failed
//!                   └─ zero-block complete ok|failed ──────▶ Completed | Failed
//! ```
//!
//! Every transition outside the table is a `PROTOCOL_VIOLATION`: a skip
//! for a path that is not `Granted`, a terminator on a lane with no
//! `Active` record for that path, a block for a record activated on
//! another socket. A lane is the in-stream control stream or one inbound
//! TCP connection (`epoch`, `socket_id`), and a lane processes records
//! sequentially, so "the active record on this lane" is unambiguous.
//!
//! `SourceDone` is valid only when nothing is `Granted` or `Active` any
//! more — the same completion rule the old `outstanding.is_empty()` gave,
//! now with the resume grants inside the same count.

use std::collections::HashMap;

use eyre::Result;

use crate::generated::FileHeader;

use super::SessionFault;

/// Which ordered lane a record rides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lane {
    /// The in-stream control stream.
    Control,
    /// One inbound TCP data-plane connection.
    DataPlane { epoch: u32, socket_id: u32 },
}

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lane::Control => write!(f, "control lane"),
            Lane::DataPlane { epoch, socket_id } => {
                write!(f, "data-plane socket {socket_id} (epoch {epoch})")
            }
        }
    }
}

/// What kind of record activated a need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecordKind {
    File,
    Resume,
    /// cr-ssc1-3: a member reserved by a tar shard record on its lane,
    /// from reservation until the shard's outcome settles it.
    Shard,
}

#[derive(Clone, Debug)]
pub(super) enum NeedState {
    /// The need went out; nothing has been announced for it.
    Granted {
        header: FileHeader,
        resume: bool,
    },
    /// A record for it is open on `lane`.
    Active {
        header: FileHeader,
        lane: Lane,
        kind: RecordKind,
    },
    Completed,
    Failed,
}

/// The ledger itself. Shared between the control loop and the data-plane
/// receive workers behind one mutex (`SharedNeedLedger`).
#[derive(Debug, Default)]
pub(super) struct NeedLedger {
    entries: HashMap<String, NeedState>,
    /// `Granted` + `Active` — what `SourceDone` must find at zero.
    open: usize,
    /// The subset of `open` that is a resume grant (`Granted{resume}` or
    /// `Active{Resume}`), for the stronger SourceDone check.
    open_resume: usize,
}

pub(super) type SharedNeedLedger = std::sync::Arc<std::sync::Mutex<NeedLedger>>;

fn violation(path: &str, message: String) -> eyre::Report {
    eyre::Report::new(SessionFault::protocol_violation(message).with_path(path))
}

impl NeedLedger {
    /// Record a fresh grant. Returns `false` when the path was already
    /// granted in this session (the dedup the old insert-only `granted`
    /// set provided): a duplicate manifest path is granted at most once.
    pub(super) fn grant(&mut self, header: FileHeader, resume: bool) -> bool {
        if self.entries.contains_key(&header.relative_path) {
            return false;
        }
        let path = header.relative_path.clone();
        self.entries
            .insert(path, NeedState::Granted { header, resume });
        self.open += 1;
        if resume {
            self.open_resume += 1;
        }
        true
    }

    /// The retained manifest header of a `Granted` need and whether it
    /// was granted with `resume`. `None` for anything else.
    pub(super) fn granted(&self, path: &str) -> Option<(&FileHeader, bool)> {
        match self.entries.get(path) {
            Some(NeedState::Granted { header, resume }) => Some((header, *resume)),
            _ => None,
        }
    }

    /// `Granted` → `Failed`: the source will not deliver this need.
    pub(super) fn skip(&mut self, path: &str) -> Result<()> {
        match self.entries.get(path) {
            Some(NeedState::Granted { resume, .. }) => {
                let resume = *resume;
                self.entries.insert(path.to_string(), NeedState::Failed);
                self.close(resume);
                Ok(())
            }
            other => Err(violation(
                path,
                format!(
                    "skip for '{path}' which is {} (a skip is valid only for a granted, \
                     unannounced need)",
                    describe(other)
                ),
            )),
        }
    }

    /// `Granted` (non-resume) → `Active(lane, File)`: a whole-file record
    /// was announced for it. Returns the retained manifest header for the
    /// caller's payload validation.
    pub(super) fn activate_file(&mut self, path: &str, lane: Lane) -> Result<FileHeader> {
        match self.entries.get(path) {
            Some(NeedState::Granted {
                header,
                resume: false,
            }) => {
                let header = header.clone();
                self.entries.insert(
                    path.to_string(),
                    NeedState::Active {
                        header: header.clone(),
                        lane,
                        kind: RecordKind::File,
                    },
                );
                Ok(header)
            }
            Some(NeedState::Granted { resume: true, .. }) => Err(violation(
                path,
                format!(
                    "file record for resume-flagged '{path}' — the contract requires its \
                     block record"
                ),
            )),
            other => Err(violation(
                path,
                format!(
                    "payload for '{path}' which is not on the need list ({})",
                    describe(other)
                ),
            )),
        }
    }

    /// The `Active(lane, kind)` header for `path`, or a violation naming
    /// what the ledger holds instead (wrong lane, wrong kind, not active).
    pub(super) fn active(&self, path: &str, lane: Lane, kind: RecordKind) -> Result<FileHeader> {
        match self.entries.get(path) {
            Some(NeedState::Active {
                header,
                lane: held,
                kind: held_kind,
            }) if *held == lane && *held_kind == kind => Ok(header.clone()),
            Some(NeedState::Active { lane: held, .. }) if *held != lane => Err(violation(
                path,
                format!("record for '{path}' on the {lane} while its record is open on the {held}"),
            )),
            other => Err(violation(
                path,
                format!(
                    "{} for '{path}' with no open record on the {lane} ({})",
                    match kind {
                        RecordKind::File => "file record terminator",
                        RecordKind::Resume => "block record",
                        RecordKind::Shard => "tar shard settlement",
                    },
                    describe(other)
                ),
            )),
        }
    }

    /// `Active(lane, File)` → `Completed` or `Failed`, from the sink's own
    /// outcome for the record: a writer that committed cleanly completes
    /// the need; one that reported the file failed (a contained
    /// destination failure, or an aborted record) fails it.
    pub(super) fn settle_file(&mut self, path: &str, lane: Lane, failed: bool) -> Result<()> {
        self.active(path, lane, RecordKind::File)?;
        self.entries.insert(
            path.to_string(),
            if failed {
                NeedState::Failed
            } else {
                NeedState::Completed
            },
        );
        self.close(false);
        Ok(())
    }

    /// cr-ssc1-3: reserve a tar shard's members atomically. Every listed
    /// path must be a distinct `Granted` (non-resume) need; validation of
    /// the members against their retained headers is the caller's,
    /// through [`Self::granted`], under the same lock and BEFORE this
    /// call. Nothing mutates unless every member passes; then every
    /// member moves `Granted` → `Active(lane, Shard)` in one step, so a
    /// second delivery of any member — on another socket, in another
    /// shard, or as a FILE/SKIP record — is a violation at the ledger,
    /// never a concurrent write to the same destination. An invalid later
    /// member still faults before any is reserved (ordered-failure
    /// behaviour kept).
    pub(super) fn reserve_shard_members(&mut self, headers: &[FileHeader], lane: Lane) -> Result<()> {
        let mut seen = std::collections::HashSet::with_capacity(headers.len());
        for header in headers {
            let path = &header.relative_path;
            if !seen.insert(path.as_str()) {
                return Err(violation(
                    path,
                    format!("tar shard lists '{path}' twice (a member is delivered once)"),
                ));
            }
            match self.entries.get(path) {
                Some(NeedState::Granted { resume: false, .. }) => {}
                Some(NeedState::Granted { resume: true, .. }) => {
                    return Err(violation(
                        path,
                        format!(
                            "tar shard entry for resume-flagged '{path}' — the contract \
                             requires its block record"
                        ),
                    ))
                }
                other => {
                    return Err(violation(
                        path,
                        format!(
                            "tar shard entry '{path}' which is not on the need list ({})",
                            describe(other)
                        ),
                    ))
                }
            }
        }
        for header in headers {
            let path = &header.relative_path;
            let Some(NeedState::Granted { header: retained, .. }) = self.entries.get(path) else {
                unreachable!("every member was verified Granted above")
            };
            let retained = retained.clone();
            self.entries.insert(
                path.clone(),
                NeedState::Active {
                    header: retained,
                    lane,
                    kind: RecordKind::Shard,
                },
            );
        }
        Ok(())
    }

    /// Settle a reserved shard's members from the sink's outcome: each
    /// must be `Active(lane, Shard)` on THIS lane (anything else is a
    /// violation — the reservation is the only way in) and goes to
    /// `Completed` or `Failed` per `failed(path)`.
    pub(super) fn settle_shard_members(
        &mut self,
        headers: &[FileHeader],
        lane: Lane,
        failed: impl Fn(&str) -> bool,
    ) -> Result<()> {
        for header in headers {
            self.active(&header.relative_path, lane, RecordKind::Shard)?;
        }
        for header in headers {
            let path = &header.relative_path;
            let state = if failed(path) {
                NeedState::Failed
            } else {
                NeedState::Completed
            };
            self.entries.insert(path.clone(), state);
            self.close(false);
        }
        Ok(())
    }

    /// A block record for a resume grant: `Granted{resume}` →
    /// `Active(lane, Resume)` on the first block, or the existing
    /// `Active(lane, Resume)` on the same lane for later blocks. Returns
    /// the retained header.
    pub(super) fn activate_resume(&mut self, path: &str, lane: Lane) -> Result<FileHeader> {
        match self.entries.get(path) {
            Some(NeedState::Granted {
                header,
                resume: true,
            }) => {
                let header = header.clone();
                self.entries.insert(
                    path.to_string(),
                    NeedState::Active {
                        header: header.clone(),
                        lane,
                        kind: RecordKind::Resume,
                    },
                );
                Ok(header)
            }
            Some(NeedState::Active { .. }) => self.active(path, lane, RecordKind::Resume),
            Some(NeedState::Granted { resume: false, .. }) => Err(violation(
                path,
                format!("block record for '{path}' which was not granted a resume-flagged need"),
            )),
            other => Err(violation(
                path,
                format!(
                    "block record for '{path}' which is not on the need list ({})",
                    describe(other)
                ),
            )),
        }
    }

    /// The header a `BlockTransferComplete` on `lane` closes: the resume
    /// grant straight from `Granted` (the zero-block record — every block
    /// matched) or the `Active(lane, Resume)` record on this lane.
    pub(super) fn resume_completing(&self, path: &str, lane: Lane) -> Result<FileHeader> {
        match self.entries.get(path) {
            Some(NeedState::Granted {
                header,
                resume: true,
            }) => Ok(header.clone()),
            Some(NeedState::Active { .. }) => self.active(path, lane, RecordKind::Resume),
            Some(NeedState::Granted { resume: false, .. }) => Err(violation(
                path,
                format!("block complete for '{path}' which was not granted a resume-flagged need"),
            )),
            other => Err(violation(
                path,
                format!(
                    "block complete for '{path}' which is not on the need list ({})",
                    describe(other)
                ),
            )),
        }
    }

    /// Close a resume grant (`Granted{resume}` or `Active(lane, Resume)`
    /// on this lane) as `Completed` or `Failed`.
    pub(super) fn settle_resume(&mut self, path: &str, lane: Lane, failed: bool) -> Result<()> {
        self.resume_completing(path, lane)?;
        self.entries.insert(
            path.to_string(),
            if failed {
                NeedState::Failed
            } else {
                NeedState::Completed
            },
        );
        self.close(true);
        Ok(())
    }

    /// Needs still `Granted` or `Active` — `SourceDone` requires zero.
    pub(super) fn open_count(&self) -> usize {
        self.open
    }

    /// The resume grants among [`Self::open_count`].
    pub(super) fn open_resume_count(&self) -> usize {
        self.open_resume
    }

    fn close(&mut self, resume: bool) {
        self.open = self.open.saturating_sub(1);
        if resume {
            self.open_resume = self.open_resume.saturating_sub(1);
        }
    }
}

fn describe(state: Option<&NeedState>) -> &'static str {
    match state {
        None => "never granted",
        Some(NeedState::Granted { resume: false, .. }) => "granted",
        Some(NeedState::Granted { resume: true, .. }) => "granted for resume",
        Some(NeedState::Active {
            kind: RecordKind::File,
            ..
        }) => "already announced as a file record",
        Some(NeedState::Active {
            kind: RecordKind::Resume,
            ..
        }) => "already open as a block record",
        Some(NeedState::Active {
            kind: RecordKind::Shard,
            ..
        }) => "already reserved by a tar shard record",
        Some(NeedState::Completed) => "already delivered",
        Some(NeedState::Failed) => "already failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(path: &str) -> FileHeader {
        FileHeader {
            relative_path: path.to_string(),
            size: 4,
            ..Default::default()
        }
    }

    const CONTROL: Lane = Lane::Control;
    const SOCK0: Lane = Lane::DataPlane {
        epoch: 0,
        socket_id: 0,
    };
    const SOCK1: Lane = Lane::DataPlane {
        epoch: 0,
        socket_id: 1,
    };

    /// cr-ssc1-3: a shard reserves its members atomically on its lane;
    /// a second delivery of any member — another socket's shard, or a
    /// duplicate inside one shard — is a violation, and the settlement
    /// must come from the reserving lane.
    #[test]
    fn shard_reservation_is_atomic_per_member_and_per_lane() {
        let mut ledger = NeedLedger::default();
        for p in ["a", "b", "c"] {
            assert!(ledger.grant(header(p), false));
        }
        // Duplicate inside one shard: rejected before anything moves.
        let dup = [header("a"), header("a")];
        let err = ledger.reserve_shard_members(&dup, SOCK0).unwrap_err();
        assert!(format!("{err:#}").contains("twice"), "{err:#}");
        assert!(
            matches!(ledger.entries.get("a"), Some(NeedState::Granted { .. })),
            "a failed reservation reserves nothing"
        );
        // An invalid later member faults before any earlier one is reserved.
        let mixed = [header("a"), header("zzz")];
        assert!(ledger.reserve_shard_members(&mixed, SOCK0).is_err());
        assert!(matches!(ledger.entries.get("a"), Some(NeedState::Granted { .. })));
        // A valid shard reserves every member on its lane.
        ledger.reserve_shard_members(&[header("a"), header("b")], SOCK0).unwrap();
        assert!(matches!(
            ledger.entries.get("a"),
            Some(NeedState::Active { lane: SOCK0, kind: RecordKind::Shard, .. })
        ));
        // Another socket cannot deliver a reserved member again.
        let err = ledger.reserve_shard_members(&[header("a")], SOCK1).unwrap_err();
        assert!(format!("{err:#}").contains("already reserved by a tar shard record"), "{err:#}");
        // Nor can a FILE record or a skip claim it.
        assert!(ledger.activate_file("a", SOCK1).is_err());
        assert!(ledger.skip("b").is_err());
        // Settlement must come from the reserving lane.
        let err = ledger
            .settle_shard_members(&[header("a"), header("b")], SOCK1, |_| false)
            .unwrap_err();
        assert!(format!("{err:#}").contains("on the"), "{err:#}");
        assert!(
            matches!(ledger.entries.get("a"), Some(NeedState::Active { .. })),
            "a rejected settlement settles nothing"
        );
        ledger
            .settle_shard_members(&[header("a"), header("b")], SOCK0, |p| p == "b")
            .unwrap();
        assert!(matches!(ledger.entries.get("a"), Some(NeedState::Completed)));
        assert!(matches!(ledger.entries.get("b"), Some(NeedState::Failed)));
        assert_eq!(ledger.open_count(), 1, "only c is still open");
        // Settling twice is a violation, not a silent no-op.
        assert!(ledger
            .settle_shard_members(&[header("a")], SOCK0, |_| false)
            .is_err());
    }

    #[test]
    fn grant_dedups_and_counts_open_needs() {
        let mut ledger = NeedLedger::default();
        assert!(ledger.grant(header("a"), false));
        assert!(
            !ledger.grant(header("a"), false),
            "duplicate grant is a no-op"
        );
        assert!(ledger.grant(header("r"), true));
        assert_eq!(ledger.open_count(), 2);
        assert_eq!(ledger.open_resume_count(), 1);
    }

    #[test]
    fn skip_is_valid_only_from_granted() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("a"), false);
        ledger.skip("a").expect("granted → failed");
        assert_eq!(ledger.open_count(), 0);
        let again = ledger.skip("a").expect_err("a second skip is a violation");
        assert!(again.to_string().contains("already failed"), "{again}");
        let never = ledger
            .skip("zzz")
            .expect_err("un-granted skip is a violation");
        assert!(never.to_string().contains("never granted"), "{never}");
    }

    #[test]
    fn file_record_activates_then_settles_on_its_own_lane() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("a"), false);
        let h = ledger
            .activate_file("a", CONTROL)
            .expect("granted → active");
        assert_eq!(h.relative_path, "a");
        let wrong_lane = ledger
            .settle_file("a", SOCK0, false)
            .expect_err("terminator on another lane is a violation");
        assert!(wrong_lane.to_string().contains("open on the control lane"));
        ledger
            .settle_file("a", CONTROL, false)
            .expect("active → completed");
        assert_eq!(ledger.open_count(), 0);
        let twice = ledger
            .activate_file("a", CONTROL)
            .expect_err("a delivered need cannot be announced again");
        assert!(twice.to_string().contains("already delivered"));
    }

    #[test]
    fn terminator_without_an_active_record_is_a_violation() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("a"), false);
        let err = ledger
            .settle_file("a", CONTROL, false)
            .expect_err("granted but never announced");
        assert!(err.to_string().contains("no open record"), "{err}");
    }

    #[test]
    fn resume_zero_block_completes_straight_from_the_grant() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("r"), true);
        ledger
            .resume_completing("r", SOCK0)
            .expect("zero-block complete from Granted");
        ledger.settle_resume("r", SOCK0, false).unwrap();
        assert_eq!(ledger.open_count(), 0);
        assert_eq!(ledger.open_resume_count(), 0);
    }

    #[test]
    fn resume_blocks_stay_on_their_socket() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("r"), true);
        ledger.activate_resume("r", SOCK0).expect("first block");
        ledger
            .activate_resume("r", SOCK0)
            .expect("later block, same socket");
        let cross = ledger
            .activate_resume("r", SOCK1)
            .expect_err("a block on another socket is a violation");
        assert!(cross.to_string().contains("socket 0"), "{cross}");
        let cross_complete = ledger
            .settle_resume("r", SOCK1, false)
            .expect_err("completion on another socket is a violation");
        assert!(cross_complete.to_string().contains("socket 0"));
        ledger.settle_resume("r", SOCK0, true).unwrap();
        assert_eq!(ledger.open_count(), 0);
    }

    #[test]
    fn file_record_for_a_resume_grant_is_refused() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("r"), true);
        let err = ledger.activate_file("r", CONTROL).unwrap_err();
        assert!(err.to_string().contains("resume-flagged"));
        let err = ledger
            .reserve_shard_members(&[header("r")], CONTROL)
            .unwrap_err();
        assert!(err.to_string().contains("resume-flagged"));
    }

    #[test]
    fn shard_members_settle_per_member() {
        let mut ledger = NeedLedger::default();
        ledger.grant(header("a"), false);
        ledger.grant(header("b"), false);
        ledger
            .reserve_shard_members(&[header("a"), header("b")], CONTROL)
            .unwrap();
        ledger
            .settle_shard_members(&[header("a"), header("b")], CONTROL, |p| p == "b")
            .unwrap();
        assert_eq!(ledger.open_count(), 0);
        assert!(matches!(
            ledger.entries.get("a"),
            Some(NeedState::Completed)
        ));
        assert!(matches!(ledger.entries.get("b"), Some(NeedState::Failed)));
    }
}
