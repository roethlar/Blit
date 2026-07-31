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
    /// Destination-side diff of a manifest chunk against the filesystem.
    ///
    /// This is the diff's WALL span and INCLUDES the sub-phases below.
    /// cr-ls1-5: it used to subtract nested attribute-repair time, which
    /// stopped being sound once checkers run concurrently — overlapping
    /// repairs can sum past the wall clock and saturate the subtraction to
    /// zero. The sub-phases are components of this number, not siblings of
    /// it; see the module docs on phases overlapping.
    Compare,
    /// The `std::fs::metadata` stat of one destination file inside the diff.
    /// Sub-phase of [`LocalPhase::Compare`], reported alongside it rather
    /// than subtracted from it — step (0) showed COMPARE owning ~100% of a
    /// converged run's wall clock, so the next question is which of its own
    /// per-file operations that is.
    CompareStat,
    /// Reading the destination's Windows metadata (durable attributes plus
    /// named-stream enumeration) to judge metadata convergence. Only runs
    /// when size/mtime already matched — which on a CONVERGED tree is every
    /// single file, so it is paid 46,041 times in the owner's case.
    CompareMetadata,
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
    pub const ALL: [LocalPhase; 10] = [
        LocalPhase::Enumerate,
        LocalPhase::EnumerateBackpressure,
        LocalPhase::Compare,
        LocalPhase::CompareStat,
        LocalPhase::CompareMetadata,
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
            LocalPhase::CompareStat => 3,
            LocalPhase::CompareMetadata => 4,
            LocalPhase::AttributeRepair => 5,
            LocalPhase::Plan => 6,
            LocalPhase::ApplyBackpressure => 7,
            LocalPhase::Apply => 8,
            LocalPhase::Delete => 9,
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
    /// The session did not complete cleanly, so at least one phase may be
    /// truncated.
    ///
    /// cr-ls1-4: the probe deliberately emits on failed runs — a run that
    /// died slowly is exactly the one worth timing — but a failure
    /// short-circuits the work inside a span, so the recorded numbers are a
    /// floor rather than a measurement. Without this flag a truncated
    /// ENUMERATE reads as "enumeration was fast" and sends attribution at
    /// the wrong phase, which is the class of error this instrument exists
    /// to avoid.
    pub session_failed: bool,
    pub phases: Vec<(LocalPhase, PhaseAggregate)>,
}

type ReportEmitter = dyn Fn(LocalPhaseReport) + Send + Sync + 'static;

struct ProbeContext {
    run_id: Arc<str>,
    phases: [AtomicPhase; 10],
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
            // cr-ls1-3: route through the shared line sink, NOT raw stderr.
            // clp-2 made the live progress row the sole owner of
            // transfer-time stderr, and an interactive run enables that row
            // automatically — a direct write would interleave with the row's
            // control sequences or be scrolled away, losing the one artifact
            // this probe exists to produce. `route_line` falls back to
            // stderr when no row is installed, so non-interactive runs are
            // unchanged.
            //
            // Deliberately not `log::info!`: that path prefixes
            // `binary: LEVEL:` and is filtered by `BLIT_LOG`, so an operator
            // running with default logging would get no artifact at all.
            //
            // Diagnostic-only: a serialization failure must never change a
            // transfer's result.
            if let Ok(line) = serde_json::to_string(&report) {
                crate::stderr_log::route_line(&format!("[local-phase] {line}"));
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

    /// Time `body` into `outer`, MINUS whatever `body` folded into `inner`
    /// while it ran.
    ///
    /// cr-ls1-2: this exists as a named operation rather than as arithmetic
    /// open-coded at each call site, because the open-coded version shipped
    /// with a guard that could not fail. Nested-cost subtraction is the one
    /// property the whole breakdown rests on — COMPARE contains
    /// ATTRIBUTE_REPAIR, and billing those nanoseconds twice would inflate
    /// exactly the phase most likely to be picked as the culprit — so it is
    /// worth having in one place with one direct test.
    ///
    /// `saturating_sub` because the two clocks are read independently; a
    /// negative remainder means a measurement fault, not a negative duration
    /// to propagate.
    /// Open the span. The caller must call [`NestedSpan::finish`] — an
    /// awaited body cannot be wrapped in a closure, so this is the form the
    /// compare seam uses.
    pub fn span_excluding(&self, outer: LocalPhase, inner: LocalPhase) -> NestedSpan<'_> {
        NestedSpan {
            probe: self,
            outer,
            inner,
            started: self.context.is_some().then(Instant::now),
            nested_before: self.total(inner),
        }
    }

    /// Closure form of [`LocalPhaseProbe::span_excluding`] for synchronous
    /// bodies.
    pub fn measure_excluding<T>(
        &self,
        outer: LocalPhase,
        inner: LocalPhase,
        body: impl FnOnce() -> T,
    ) -> T {
        let span = self.span_excluding(outer, inner);
        let out = body();
        span.finish();
        out
    }

    /// Emit the breakdown exactly once per session. A second call is ignored
    /// so a retry or an error path cannot double-report.
    pub fn emit(&self, session_wall: Duration, session_failed: bool) {
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
            session_failed,
            phases: LocalPhase::ALL
                .iter()
                .map(|phase| (*phase, context.phases[phase.index()].snapshot()))
                .collect(),
        };
        (context.emit)(report);
    }
}

/// An open outer span that will subtract whatever lands in its inner phase
/// before it finishes. Created by [`LocalPhaseProbe::span_excluding`].
///
/// Deliberately NOT `Drop`-based: an early return or a `?` would then record
/// a span the caller never meant to close, and a diagnostic that silently
/// records partial spans is worse than one that records nothing.
pub struct NestedSpan<'a> {
    probe: &'a LocalPhaseProbe,
    outer: LocalPhase,
    inner: LocalPhase,
    started: Option<Instant>,
    nested_before: Duration,
}

impl NestedSpan<'_> {
    /// Close the span, recording `elapsed - nested` into the outer phase.
    pub fn finish(self) {
        let Some(started) = self.started else {
            return;
        };
        let nested = self
            .probe
            .total(self.inner)
            .saturating_sub(self.nested_before);
        self.probe
            .record(self.outer, started.elapsed().saturating_sub(nested));
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
        probe.emit(Duration::from_secs(2), false);
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
        probe.emit(Duration::from_millis(100), false);

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
        probe.emit(Duration::from_millis(5), false);
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
        probe.emit(Duration::from_millis(100), false);

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
        probe.emit(Duration::from_millis(1), false);
        probe.emit(Duration::from_millis(1), false);
        assert_eq!(sink.lock().expect("sink poisoned").len(), 1);
    }

    #[test]
    fn clones_share_one_accumulator() {
        let (probe, sink) = capturing();
        let clone = probe.clone();
        probe.record(LocalPhase::Apply, Duration::from_millis(1));
        clone.record(LocalPhase::Apply, Duration::from_millis(1));
        clone.emit(Duration::from_millis(10), false);

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

    /// cr-ls1-2: the subtraction guard that actually bites.
    ///
    /// The trick is to make the nested phase claim FAR more time than the
    /// outer span could possibly have taken. With subtraction, the outer
    /// saturates to exactly zero. Without it, the outer records its real
    /// elapsed time, which is non-zero. That is a categorical difference, not
    /// a timing comparison, so the assertion cannot pass by luck on a fast
    /// machine — which is exactly how the previous guard
    /// (`compare + repair <= 2 * wall`) managed to hold whether or not the
    /// subtraction existed.
    #[test]
    fn nested_time_is_subtracted_from_the_enclosing_span() {
        let (probe, sink) = capturing();
        probe.measure_excluding(LocalPhase::Compare, LocalPhase::AttributeRepair, || {
            // An hour of "nested" work inside a span that really takes
            // microseconds.
            probe.record(LocalPhase::AttributeRepair, Duration::from_secs(3600));
        });
        probe.emit(Duration::from_millis(1), false);

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        let compare = report
            .phases
            .iter()
            .find(|(phase, _)| *phase == LocalPhase::Compare)
            .map(|(_, aggregate)| aggregate)
            .expect("compare present");
        assert_eq!(
            compare.total_ns, 0,
            "the nested hour must be subtracted; without the subtraction this \
             records the span's real elapsed time instead"
        );
    }

    #[test]
    fn only_nested_time_from_inside_the_span_is_subtracted() {
        // Work recorded to the inner phase BEFORE the span opens belongs to
        // an earlier span and must not be deducted from this one — otherwise
        // one chunk's repairs would erase the next chunk's compare cost.
        let (probe, sink) = capturing();
        probe.record(LocalPhase::AttributeRepair, Duration::from_secs(3600));
        probe.measure_excluding(LocalPhase::Compare, LocalPhase::AttributeRepair, || {
            std::thread::sleep(Duration::from_millis(5));
        });
        probe.emit(Duration::from_millis(10), false);

        let reports = sink.lock().expect("sink poisoned");
        let compare = reports
            .first()
            .expect("one report")
            .phases
            .iter()
            .find(|(phase, _)| *phase == LocalPhase::Compare)
            .map(|(_, aggregate)| aggregate.clone())
            .expect("compare present");
        assert!(
            compare.total_ns > 0,
            "prior nested time must not be deducted from a later span"
        );
    }

    #[test]
    fn a_span_on_a_disabled_probe_records_nothing_and_does_not_panic() {
        let probe = LocalPhaseProbe::disabled();
        probe.measure_excluding(LocalPhase::Compare, LocalPhase::AttributeRepair, || {});
        probe.span_excluding(LocalPhase::Compare, LocalPhase::AttributeRepair);
        assert_eq!(probe.total(LocalPhase::Compare), Duration::ZERO);
    }

    /// cr-ls1-4: a failed session's report must SAY it is truncated.
    #[test]
    fn a_failed_session_is_marked_in_the_report() {
        let (probe, sink) = capturing();
        probe.emit(Duration::from_millis(1), true);
        let reports = sink.lock().expect("sink poisoned");
        assert!(
            reports.first().expect("one report").session_failed,
            "a truncated report that does not admit it reads as a fast one"
        );
    }

    #[test]
    fn a_clean_session_is_not_marked_failed() {
        let (probe, sink) = capturing();
        probe.emit(Duration::from_millis(1), false);
        let reports = sink.lock().expect("sink poisoned");
        assert!(!reports.first().expect("one report").session_failed);
    }

    /// cr-ls1-3: the artifact is one line, machine-readable, and carries its
    /// marker — the properties that make routing it through the row-aware
    /// sink worth doing rather than writing raw stderr.
    #[test]
    fn the_report_serializes_to_one_parseable_line() {
        let (probe, sink) = capturing();
        probe.record(LocalPhase::Compare, Duration::from_millis(3));
        probe.emit(Duration::from_millis(9), false);

        let reports = sink.lock().expect("sink poisoned");
        let json = serde_json::to_string(reports.first().expect("one report"))
            .expect("the report serializes");
        assert!(!json.contains('\n'), "one line, so a row cannot split it");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["schema"], 1);
        assert_eq!(parsed["session_failed"], false);
        assert_eq!(parsed["phases_overlap"], true);
        // Phase names are stable identifiers, not Debug output.
        assert!(json.contains("APPLY_BACKPRESSURE"));
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
