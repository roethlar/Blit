//! Default-off timing records for the low-frequency transfer lifecycle.
//!
//! These records surround session establishment and command handling. They are
//! deliberately separate from the schema-1 `[session-phase]` stream, which
//! remains the detailed observer for an established transfer session.

use super::session_phase::{session_trace_id, trace_env_run_id, SessionPhaseRole};
use serde::Serialize;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Result recorded on a lifecycle boundary that completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferLifecycleOutcome {
    Success,
    Refused,
    Error,
}

/// One structured lifecycle timing record.
#[derive(Clone, Debug, Serialize)]
pub struct TransferLifecycleEvent {
    pub schema: u8,
    pub run_id: String,
    pub producer_seq: u64,
    pub unix_ns: u128,
    pub elapsed_ns: u128,
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator_role: Option<SessionPhaseRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TransferLifecycleOutcome>,
}

type EventEmitter = dyn Fn(TransferLifecycleEvent) + Send + Sync + 'static;
type FlushEmitter = dyn Fn() + Send + Sync + 'static;

enum LifecycleWriterOutput {
    Line(String),
    Flush,
}

struct LifecycleContext {
    run_id: Arc<str>,
    origin: Instant,
    producer_seq: AtomicU64,
    initiator_role: OnceLock<SessionPhaseRole>,
    session_id: OnceLock<Arc<str>>,
    emit: Arc<EventEmitter>,
    flush: Arc<FlushEmitter>,
}

/// Explicit, cloneable context for one initiating transfer process.
///
/// [`TransferLifecycleTrace::from_env`] is the production constructor. It
/// stays disabled unless the existing `BLIT_TRACE_SESSION_PHASES` flag is
/// truthy, and uses `BLIT_TRACE_RUN_ID` for cross-process correlation.
#[derive(Clone, Default)]
pub struct TransferLifecycleTrace {
    context: Option<Arc<LifecycleContext>>,
}

impl TransferLifecycleTrace {
    /// Create the production context at the earliest observable caller stamp.
    pub fn from_env() -> Self {
        Self::from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
    }

    /// Build a deterministic capture context without consulting process
    /// environment or starting a writer thread.
    pub fn capture(
        run_id: impl Into<String>,
        emit: impl Fn(TransferLifecycleEvent) + Send + Sync + 'static,
    ) -> Self {
        Self::capture_with_flush(run_id, emit, || {})
    }

    /// Build a deterministic capture context with an observable flush hook.
    pub fn capture_with_flush(
        run_id: impl Into<String>,
        emit: impl Fn(TransferLifecycleEvent) + Send + Sync + 'static,
        flush: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self::with_emitters(
            run_id.into(),
            Instant::now(),
            Arc::new(emit),
            Arc::new(flush),
        )
    }

    /// Force lifecycle tracing off without consulting process environment.
    pub fn disabled() -> Self {
        Self { context: None }
    }

    /// Whether this context will emit lifecycle records.
    pub fn is_enabled(&self) -> bool {
        self.context.is_some()
    }

    /// Attach the semantic role once route selection makes it known. The first
    /// attachment wins so clones cannot race the context into inconsistent
    /// correlation fields.
    pub fn attach_initiator_role(&self, role: SessionPhaseRole) {
        if let Some(context) = &self.context {
            let _ = context.initiator_role.set(role);
        }
    }

    /// Attach the derived, non-secret session correlation ID once the session
    /// token is known. The raw token is never retained or emitted.
    pub fn attach_session_token(&self, session_token: &[u8]) {
        if let Some(context) = &self.context {
            let _ = context.session_id.set(session_trace_id(session_token));
        }
    }

    /// Record one low-frequency lifecycle boundary.
    pub fn record(&self, event: &'static str, outcome: Option<TransferLifecycleOutcome>) {
        let Some(context) = &self.context else {
            return;
        };
        let now = Instant::now();
        let record = TransferLifecycleEvent {
            schema: 1,
            run_id: context.run_id.to_string(),
            producer_seq: context.producer_seq.fetch_add(1, Ordering::Relaxed),
            unix_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            elapsed_ns: now.saturating_duration_since(context.origin).as_nanos(),
            event,
            initiator_role: context.initiator_role.get().copied(),
            session_id: context.session_id.get().map(ToString::to_string),
            outcome,
        };
        (context.emit)(record);
    }

    /// Flush records already admitted to the asynchronous writer. Writer
    /// failure remains diagnostic-only and never changes a product result.
    pub fn flush(&self) {
        if let Some(context) = &self.context {
            (context.flush)();
        }
    }

    /// Flush admitted records without blocking an async runtime worker on the
    /// writer's rendezvous. Join failure remains diagnostic-only.
    pub async fn flush_async(&self) {
        if !self.is_enabled() {
            return;
        }
        let trace = self.clone();
        let _ = tokio::task::spawn_blocking(move || trace.flush()).await;
    }

    fn from_env_with(
        read: impl FnMut(&str) -> Option<String>,
        writer: impl FnOnce(String, Instant) -> Self,
    ) -> Self {
        let origin = Instant::now();
        let Some(run_id) = trace_env_run_id(read) else {
            return Self::disabled();
        };
        writer(run_id, origin)
    }

    fn stderr_writer(run_id: String, origin: Instant) -> Self {
        let stderr = std::io::stderr();
        Self::threaded_writer(run_id, origin, move |output| {
            let mut stderr = stderr.lock();
            match output {
                LifecycleWriterOutput::Line(line) => {
                    let _ = writeln!(stderr, "{line}");
                }
                LifecycleWriterOutput::Flush => {
                    let _ = stderr.flush();
                }
            }
        })
    }

    fn threaded_writer(
        run_id: String,
        origin: Instant,
        mut output: impl FnMut(LifecycleWriterOutput) + Send + 'static,
    ) -> Self {
        enum WriterMessage {
            Event(Box<TransferLifecycleEvent>),
            Flush(std::sync::mpsc::SyncSender<()>),
        }

        let (tx, rx) = std::sync::mpsc::channel::<WriterMessage>();
        let spawned = std::thread::Builder::new()
            .name("blit-transfer-lifecycle".into())
            .spawn(move || {
                while let Ok(message) = rx.recv() {
                    match message {
                        WriterMessage::Event(event) => {
                            output(LifecycleWriterOutput::Line(json_line(&event)));
                        }
                        WriterMessage::Flush(done) => {
                            output(LifecycleWriterOutput::Flush);
                            let _ = done.send(());
                        }
                    }
                }
            });
        if spawned.is_err() {
            return Self::disabled();
        }

        let event_tx = tx.clone();
        let flush_tx = tx;
        Self::with_emitters(
            run_id,
            origin,
            Arc::new(move |event| {
                let _ = event_tx.send(WriterMessage::Event(Box::new(event)));
            }),
            Arc::new(move || {
                let (done_tx, done_rx) = std::sync::mpsc::sync_channel(0);
                if flush_tx.send(WriterMessage::Flush(done_tx)).is_ok() {
                    let _ = done_rx.recv();
                }
            }),
        )
    }

    fn with_emitters(
        run_id: String,
        origin: Instant,
        emit: Arc<EventEmitter>,
        flush: Arc<FlushEmitter>,
    ) -> Self {
        Self {
            context: Some(Arc::new(LifecycleContext {
                run_id: Arc::from(run_id),
                origin,
                producer_seq: AtomicU64::new(0),
                initiator_role: OnceLock::new(),
                session_id: OnceLock::new(),
                emit,
                flush,
            })),
        }
    }
}

fn json_line(event: &TransferLifecycleEvent) -> String {
    match serde_json::to_string(event) {
        Ok(line) => format!("[transfer-lifecycle] {line}"),
        Err(err) => format!("[transfer-lifecycle] serialization_error={err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Mutex;

    #[test]
    fn trace_off_never_constructs_a_writer_or_calls_emitters() {
        let writers = Arc::new(AtomicUsize::new(0));
        let writer_count = Arc::clone(&writers);
        let trace = TransferLifecycleTrace::from_env_with(
            |_| Some("off".into()),
            move |_, _| {
                writer_count.fetch_add(1, AtomicOrdering::Relaxed);
                TransferLifecycleTrace::capture("unexpected", |_| {})
            },
        );

        assert!(!trace.is_enabled());
        trace.attach_initiator_role(SessionPhaseRole::Source);
        trace.attach_session_token(b"disabled-session");
        trace.record("must_not_emit", Some(TransferLifecycleOutcome::Error));
        trace.flush();
        assert_eq!(writers.load(AtomicOrdering::Relaxed), 0);
    }

    #[test]
    fn capture_correlates_order_role_session_and_terminal_outcome() {
        let events: Arc<Mutex<Vec<TransferLifecycleEvent>>> = Arc::default();
        let captured = Arc::clone(&events);
        let flushes = Arc::new(AtomicUsize::new(0));
        let captured_flushes = Arc::clone(&flushes);
        let trace = TransferLifecycleTrace::capture_with_flush(
            "unit-run",
            move |event| captured.lock().unwrap().push(event),
            move || {
                captured_flushes.fetch_add(1, AtomicOrdering::Relaxed);
            },
        );

        trace.record("async_main_enter", None);
        trace.attach_initiator_role(SessionPhaseRole::Destination);
        trace.attach_session_token(b"never-log-this-session-token");
        trace.attach_session_token(b"later-token-must-not-replace-the-first");
        trace.record("command_terminal", Some(TransferLifecycleOutcome::Success));
        trace.flush();

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].producer_seq, 0);
        assert_eq!(events[1].producer_seq, 1);
        assert!(events[0].elapsed_ns <= events[1].elapsed_ns);
        assert_eq!(events[0].run_id, "unit-run");
        assert_eq!(events[0].initiator_role, None);
        assert_eq!(events[0].session_id, None);
        assert_eq!(
            events[1].initiator_role,
            Some(SessionPhaseRole::Destination)
        );
        assert_eq!(
            events[1].session_id.as_deref(),
            Some(session_trace_id(b"never-log-this-session-token").as_ref())
        );
        assert_eq!(events[1].outcome, Some(TransferLifecycleOutcome::Success));
        assert_eq!(flushes.load(AtomicOrdering::Relaxed), 1);
    }

    #[test]
    fn clones_share_one_concurrency_safe_producer_sequence() {
        let events: Arc<Mutex<Vec<TransferLifecycleEvent>>> = Arc::default();
        let captured = Arc::clone(&events);
        let trace = TransferLifecycleTrace::capture("parallel-run", move |event| {
            captured.lock().unwrap().push(event);
        });
        let threads = (0..16)
            .map(|_| {
                let trace = trace.clone();
                std::thread::spawn(move || trace.record("parallel_boundary", None))
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }

        let mut sequences = events
            .lock()
            .unwrap()
            .iter()
            .map(|event| event.producer_seq)
            .collect::<Vec<_>>();
        sequences.sort_unstable();
        assert_eq!(sequences, (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn threaded_writer_uses_its_own_prefix_and_flushes_admitted_records() {
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let flushes = Arc::new(AtomicUsize::new(0));
        let writer_lines = Arc::clone(&lines);
        let writer_flushes = Arc::clone(&flushes);
        let trace = TransferLifecycleTrace::threaded_writer(
            "writer-run".into(),
            Instant::now(),
            move |output| match output {
                LifecycleWriterOutput::Line(line) => writer_lines.lock().unwrap().push(line),
                LifecycleWriterOutput::Flush => {
                    writer_flushes.fetch_add(1, AtomicOrdering::Relaxed);
                }
            },
        );
        trace.attach_initiator_role(SessionPhaseRole::Source);
        trace.record("probe", Some(TransferLifecycleOutcome::Refused));
        trace.flush();

        assert_eq!(flushes.load(AtomicOrdering::Relaxed), 1);
        let line = lines.lock().unwrap().pop().unwrap();
        assert!(line.starts_with("[transfer-lifecycle] "));
        assert!(!line.starts_with("[session-phase] "));
        let json = line.strip_prefix("[transfer-lifecycle] ").unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["run_id"], "writer-run");
        assert_eq!(value["initiator_role"], "SOURCE");
        assert_eq!(value["outcome"], "REFUSED");
    }
}
