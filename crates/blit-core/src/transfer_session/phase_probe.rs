//! Default-off wall-clock breakdown for the LOCAL transfer route (ls-1).
//!
//! `docs/plan/LOCAL_SMALL_FILE_PATH.md` slice ls-1 step (0): before anything
//! attributes cost *inside* a phase, something has to say which phase owns the
//! wall clock. The owner's 2026-07-31 field check is the reason — a mirror that
//! copied 0 files and 0 bytes still took 283.92 s, and every hypothesis that
//! plan carries (L1..L4) is an *apply*-path hypothesis. If enumerate/compare
//! dominates, attributing inside apply answers the wrong question.
//!
//! Deliberately NOT the remote small-file probe. `SmallFileProbe` is switched
//! off for local-apply sessions on purpose (`713526e8`, finding
//! `otp12-pf1-p2-observer.md`: "local payload bytes do not ride either observed
//! remote carrier"), and that exclusion is correct — its shard receive/sink
//! records are keyed to the TCP and in-stream carriers, which the local route
//! does not use. This probe measures the local route's own phases and leaves
//! that exclusion intact.
//!
//! # Phases overlap; they are not a pie chart
//!
//! `run_local_session` joins the SOURCE and DESTINATION drivers concurrently
//! (`tokio::join!`), so [`LocalPhase::Enumerate`] runs *while*
//! [`LocalPhase::Compare`] runs. The report therefore carries
//! `session_wall_ns` alongside the per-phase spans and callers must read each
//! phase as a share of that wall figure, never as a partition of it. Summing
//! the phases and expecting the wall total is a reader error this module
//! refuses to encourage: see `phases_do_not_partition_wall_time`.

use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TRACE_ENV: &str = "BLIT_TRACE_LOCAL_PHASES";
const RUN_ID_ENV: &str = "BLIT_TRACE_RUN_ID";

/// One measured phase of a local session.
///
/// `AttributeRepair` is carved out of `Compare` rather than nested under it:
/// pfc-6 repairs attributes in place at diff time, so a converged-but-for-
/// attributes tree spends compare-phase time doing destination *writes*. The
/// field check's 5445 repairs are exactly that cost, and folding them into
/// `Compare` would hide the one term the owner's run made visible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LocalPhase {
    /// Source-side filesystem walk producing manifest headers, EXCLUDING the
    /// time the walk spends blocked on the manifest channel.
    Enumerate,
    /// Time the source walk spent blocked handing a header downstream. Split
    /// out of [`LocalPhase::Enumerate`] deliberately: the manifest channel is
    /// bounded, so a destination that diffs slowly (an SMB target, say) stalls
    /// the walk, and folding that wait into `Enumerate` would report the
    /// destination's cost as the source's. Misattributing exactly this way is
    /// what ls-1 step (0) exists to prevent.
    EnumerateBackpressure,
    /// Destination-side diff of a manifest chunk against the filesystem,
    /// excluding the attribute repairs it may trigger.
    Compare,
    /// pfc-6 in-place attribute repair performed during the diff.
    AttributeRepair,
    /// Turning diff verdicts into payloads (planner + shard assembly).
    Plan,
    /// Time the diff loop spent blocked handing a payload to the apply
    /// pipeline's bounded queue. This is where a SLOW SINK actually shows up.
    ///
    /// cr-ls1-1: the first cut of this module measured only [`LocalPhase::Apply`]
    /// (the tail drain) and left this wait inside no span at all, which meant
    /// a sink slower than the planner — the expected shape on an SMB
    /// destination — could dominate the wall clock while APPLY reported near
    /// zero. A phase breakdown that can lose the dominant cost is worse than
    /// none, because it reads as authoritative.
    ApplyBackpressure,
    /// Apply DRAIN: from the moment the diff stops queueing payloads to the
    /// moment the apply pipeline finishes. Not total apply cost — the
    /// pipeline runs concurrently with the diff. Read together with
    /// [`LocalPhase::ApplyBackpressure`]: a long drain means the writer was
    /// still working after the reader finished, a long backpressure means the
    /// writer was pacing the reader throughout. Instrumenting inside the
    /// pipeline itself would mean editing `execute_sink_pipeline_streaming`,
    /// which the remote routes share; ls-1 measures the local route without
    /// changing shared code.
    Apply,
    /// The mirror delete pass.
    Delete,
}

impl LocalPhase {
    /// Every phase, in the order a session encounters them. Fixed so the
    /// report's field order is stable across runs and diffable.
    pub const ALL: [LocalPhase; 8] = [
        LocalPhase::Enumerate,
        LocalPhase::EnumerateBackpressure,
        LocalPhase::Compare,
        LocalPhase::AttributeRepair,
        LocalPhase::Plan,
        LocalPhase::ApplyBackpressure,
        LocalPhase::Apply,
        LocalPhase::Delete,
    ];

    fn index(self) -> usize {
        match self {
            LocalPhase::Enumerate => 0,
            LocalPhase::EnumerateBackpressure => 1,
            LocalPhase::Compare => 2,
            LocalPhase::AttributeRepair => 3,
            LocalPhase::Plan => 4,
            LocalPhase::ApplyBackpressure => 5,
            LocalPhase::Apply => 6,
            LocalPhase::Delete => 7,
        }
    }
}

/// Aggregate for one phase. Bounded by construction: three counters, no
/// per-sample retention, so a 9578-file run costs the same memory as a
/// one-file run.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PhaseAggregate {
    pub samples: u64,
    pub total_ns: u64,
    pub max_ns: u64,
}

#[derive(Debug, Default)]
struct AtomicPhase {
    samples: AtomicU64,
    total_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl AtomicPhase {
    fn record(&self, duration: Duration) {
        let ns = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        self.samples.fetch_add(1, Ordering::Relaxed);
        self.total_ns.fetch_add(ns, Ordering::Relaxed);
        self.max_ns.fetch_max(ns, Ordering::Relaxed);
    }

    fn snapshot(&self) -> PhaseAggregate {
        PhaseAggregate {
            samples: self.samples.load(Ordering::Relaxed),
            total_ns: self.total_ns.load(Ordering::Relaxed),
            max_ns: self.max_ns.load(Ordering::Relaxed),
        }
    }
}

/// One emitted breakdown. `session_wall_ns` is the denominator; the phase
/// spans are concurrent and do not sum to it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LocalPhaseReport {
    pub schema: u8,
    pub run_id: String,
    pub unix_ns: u128,
    pub session_wall_ns: u64,
    /// Present so a reader cannot mistake the spans for a partition without
    /// having been told otherwise in the artifact itself.
    pub phases_overlap: bool,
    pub phases: Vec<(LocalPhase, PhaseAggregate)>,
}

type ReportEmitter = dyn Fn(LocalPhaseReport) + Send + Sync + 'static;

struct ProbeContext {
    run_id: Arc<str>,
    phases: [AtomicPhase; 8],
    emit: Arc<ReportEmitter>,
    emitted: OnceLock<()>,
}

/// Default-off local phase probe. Clone-cheap; all clones share one
/// accumulator.
///
/// `Default` permits environment activation but installs no emitter — the same
/// shape as [`crate::remote::transfer::SmallFileProbe`], so a caller that never
/// mentions the probe still gets production tracing when an operator asks for
/// it, while [`LocalPhaseProbe::capture`] and [`LocalPhaseProbe::disabled`]
/// pin a test's behaviour regardless of the ambient environment.
#[derive(Clone)]
pub struct LocalPhaseProbe {
    context: Option<Arc<ProbeContext>>,
    allow_env: bool,
}

/// Hand-written because the emitter closure is not `Debug`. Reports whether
/// the probe is live rather than trying to describe the sink.
impl std::fmt::Debug for LocalPhaseProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalPhaseProbe")
            .field("enabled", &self.context.is_some())
            .field("allow_env", &self.allow_env)
            .finish()
    }
}

impl Default for LocalPhaseProbe {
    fn default() -> Self {
        Self {
            context: None,
            allow_env: true,
        }
    }
}

impl LocalPhaseProbe {
    /// Resolve an unbound probe against the environment. A probe that already
    /// carries an emitter, or that was explicitly disabled, is returned
    /// untouched so tests never pick up ambient tracing.
    pub(crate) fn or_from_env(self) -> Self {
        if self.context.is_some() || !self.allow_env {
            return self;
        }
        Self::from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
    }

    fn from_env_with(
        mut read: impl FnMut(&str) -> Option<String>,
        writer: impl FnOnce(String) -> Self,
    ) -> Self {
        let enabled = read(TRACE_ENV).is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        });
        if !enabled {
            return Self::disabled();
        }
        let Some(run_id) = read(RUN_ID_ENV).filter(|value| !value.trim().is_empty()) else {
            eprintln!("[local-phase-probe] configuration_error={RUN_ID_ENV} must be non-empty");
            return Self::disabled();
        };
        writer(run_id)
    }

    fn stderr_writer(run_id: String) -> Self {
        Self::capture(run_id, |report| {
            // Diagnostic-only: a serialization or write failure must never
            // change a transfer's result.
            if let Ok(line) = serde_json::to_string(&report) {
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "[local-phase] {line}");
                let _ = err.flush();
            }
        })
    }

    /// Deterministic in-memory constructor for tests: no environment read, no
    /// stderr write.
    pub fn capture(
        run_id: impl Into<String>,
        emit: impl Fn(LocalPhaseReport) + Send + Sync + 'static,
    ) -> Self {
        Self {
            context: Some(Arc::new(ProbeContext {
                run_id: Arc::from(run_id.into()),
                phases: Default::default(),
                emit: Arc::new(emit),
                emitted: OnceLock::new(),
            })),
            allow_env: false,
        }
    }

    /// Force the probe off without consulting the environment.
    pub fn disabled() -> Self {
        Self {
            context: None,
            allow_env: false,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.context.is_some()
    }

    /// Fold one measured duration into a phase.
    pub fn record(&self, phase: LocalPhase, duration: Duration) {
        if let Some(context) = &self.context {
            context.phases[phase.index()].record(duration);
        }
    }

    /// Total time folded into `phase` so far. Used to derive one phase from
    /// another (the enumerate walk subtracts its own backpressure); zero when
    /// the probe is off.
    pub fn total(&self, phase: LocalPhase) -> Duration {
        match &self.context {
            Some(context) => Duration::from_nanos(
                context.phases[phase.index()]
                    .total_ns
                    .load(Ordering::Relaxed),
            ),
            None => Duration::ZERO,
        }
    }

    /// Time `body` into `phase`. Returns whatever `body` returns, so a call
    /// site wraps without restructuring — and, when the probe is off, without
    /// reading the clock at all.
    pub fn measure<T>(&self, phase: LocalPhase, body: impl FnOnce() -> T) -> T {
        if self.context.is_none() {
            return body();
        }
        let started = Instant::now();
        let out = body();
        self.record(phase, started.elapsed());
        out
    }

    /// Emit the breakdown exactly once per session. A second call is ignored
    /// so a retry or an error path cannot double-report.
    pub fn emit(&self, session_wall: Duration) {
        let Some(context) = &self.context else {
            return;
        };
        if context.emitted.set(()).is_err() {
            return;
        }
        let report = LocalPhaseReport {
            schema: 1,
            run_id: context.run_id.to_string(),
            unix_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            session_wall_ns: u64::try_from(session_wall.as_nanos()).unwrap_or(u64::MAX),
            phases_overlap: true,
            phases: LocalPhase::ALL
                .iter()
                .map(|phase| (*phase, context.phases[phase.index()].snapshot()))
                .collect(),
        };
        (context.emit)(report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn capturing() -> (LocalPhaseProbe, Arc<Mutex<Vec<LocalPhaseReport>>>) {
        let sink: Arc<Mutex<Vec<LocalPhaseReport>>> = Arc::default();
        let seen = Arc::clone(&sink);
        let probe = LocalPhaseProbe::capture("run-1", move |report| {
            seen.lock().expect("sink poisoned").push(report);
        });
        (probe, sink)
    }

    #[test]
    fn disabled_probe_records_and_emits_nothing() {
        let probe = LocalPhaseProbe::disabled();
        assert!(!probe.is_enabled());
        probe.record(LocalPhase::Compare, Duration::from_secs(1));
        probe.emit(Duration::from_secs(2));
        // Nothing to assert against but the absence of a panic and of an
        // emitter: the default probe holds no context to emit through.
        assert!(probe.context.is_none());
    }

    #[test]
    fn measure_returns_the_body_value_when_disabled() {
        let probe = LocalPhaseProbe::disabled();
        assert_eq!(probe.measure(LocalPhase::Apply, || 41 + 1), 42);
    }

    #[test]
    fn aggregates_fold_samples_total_and_max() {
        let (probe, sink) = capturing();
        probe.record(LocalPhase::Compare, Duration::from_millis(10));
        probe.record(LocalPhase::Compare, Duration::from_millis(30));
        probe.record(LocalPhase::Compare, Duration::from_millis(20));
        probe.emit(Duration::from_millis(100));

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        let compare = report
            .phases
            .iter()
            .find(|(phase, _)| *phase == LocalPhase::Compare)
            .map(|(_, aggregate)| aggregate)
            .expect("compare present");
        assert_eq!(compare.samples, 3);
        assert_eq!(
            compare.total_ns,
            Duration::from_millis(60).as_nanos() as u64
        );
        assert_eq!(compare.max_ns, Duration::from_millis(30).as_nanos() as u64);
    }

    #[test]
    fn every_phase_appears_even_when_never_entered() {
        let (probe, sink) = capturing();
        probe.emit(Duration::from_millis(5));
        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        assert_eq!(report.phases.len(), LocalPhase::ALL.len());
        // A phase that never ran must read as a measured zero, not as a
        // missing field: "apply took no time" and "apply was not measured"
        // are different findings and ls-1 has to tell them apart.
        for (_, aggregate) in &report.phases {
            assert_eq!(aggregate.samples, 0);
            assert_eq!(aggregate.total_ns, 0);
        }
    }

    #[test]
    fn phases_do_not_partition_wall_time() {
        // The concurrency contract, pinned: enumerate and compare overlap, so
        // their spans can exceed the session wall clock. A reader (or a later
        // slice) that assumes a partition is wrong, and this test is the
        // statement of that.
        let (probe, sink) = capturing();
        probe.record(LocalPhase::Enumerate, Duration::from_millis(80));
        probe.record(LocalPhase::Compare, Duration::from_millis(80));
        probe.emit(Duration::from_millis(100));

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        let summed: u64 = report
            .phases
            .iter()
            .map(|(_, aggregate)| aggregate.total_ns)
            .sum();
        assert!(summed > report.session_wall_ns);
        assert!(report.phases_overlap);
    }

    #[test]
    fn emit_is_once_per_session() {
        let (probe, sink) = capturing();
        probe.emit(Duration::from_millis(1));
        probe.emit(Duration::from_millis(1));
        assert_eq!(sink.lock().expect("sink poisoned").len(), 1);
    }

    #[test]
    fn clones_share_one_accumulator() {
        let (probe, sink) = capturing();
        let clone = probe.clone();
        probe.record(LocalPhase::Apply, Duration::from_millis(1));
        clone.record(LocalPhase::Apply, Duration::from_millis(1));
        clone.emit(Duration::from_millis(10));

        let reports = sink.lock().expect("sink poisoned");
        let apply = reports
            .first()
            .expect("one report")
            .phases
            .iter()
            .find(|(phase, _)| *phase == LocalPhase::Apply)
            .map(|(_, aggregate)| aggregate)
            .expect("apply present");
        assert_eq!(apply.samples, 2);
    }

    #[test]
    fn capture_and_disabled_ignore_the_environment() {
        // An explicitly constructed probe must not be re-resolved against the
        // ambient environment: a test that pinned its behaviour keeps it even
        // if the operator's shell has tracing switched on.
        assert!(!LocalPhaseProbe::disabled().or_from_env().is_enabled());
        let captured = LocalPhaseProbe::capture("run-1", |_| {});
        assert!(captured.or_from_env().is_enabled());
    }

    #[test]
    fn env_gate_needs_both_keys() {
        let off = LocalPhaseProbe::from_env_with(|_| None, |_| unreachable!("must not construct"));
        assert!(!off.is_enabled());

        // Flag on, run ID missing: refuse rather than emit uncorrelatable
        // records into a benchmark directory.
        let no_run_id = LocalPhaseProbe::from_env_with(
            |name| (name == TRACE_ENV).then(|| "1".to_string()),
            |_| unreachable!("must not construct without a run ID"),
        );
        assert!(!no_run_id.is_enabled());

        let on = LocalPhaseProbe::from_env_with(
            |name| match name {
                TRACE_ENV => Some("1".to_string()),
                RUN_ID_ENV => Some("run-7".to_string()),
                _ => None,
            },
            |run_id| LocalPhaseProbe::capture(run_id, |_| {}),
        );
        assert!(on.is_enabled());
    }

    #[test]
    fn blank_run_id_is_rejected_like_a_missing_one() {
        let probe = LocalPhaseProbe::from_env_with(
            |name| match name {
                TRACE_ENV => Some("yes".to_string()),
                RUN_ID_ENV => Some("   ".to_string()),
                _ => None,
            },
            |_| unreachable!("must not construct on a blank run ID"),
        );
        assert!(!probe.is_enabled());
    }
}
