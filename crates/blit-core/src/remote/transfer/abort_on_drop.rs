//! RAII wrapper that aborts the underlying tokio task when dropped
//! without awaiting (R32-F2; hoisted to a shared location under
//! `w4-1` so every spawn family — not just `pull.rs` — can use it).
//!
//! `JoinHandle::drop` detaches; it does NOT cancel the spawned task.
//! That's a real bug wherever a spawned task's lifetime is meant to
//! be bounded by a calling future: when the outer future is dropped
//! (e.g. CLI Ctrl-C cancels the gRPC stream from the daemon's
//! `delegated_pull` handler, or an early `?` return exits a handler
//! while a data-plane task is still running), a bare `JoinHandle`
//! left running would otherwise keep reading sockets and writing
//! files with no owner.
//!
//! Usage: wrap every `tokio::spawn` whose lifetime should be bounded
//! by the calling future. Await with `.join().await` — that holds
//! `self` across the await so a parent-future cancellation during
//! the await still triggers `abort()` via Drop. Do NOT add an
//! `into_inner()` accessor: returning the bare `JoinHandle` and then
//! awaiting it re-introduces the cancellation gap (R34-F2 — the bare
//! handle is dropped on parent-future cancel and detaches the task
//! instead of aborting it).

use tokio::task::JoinHandle;

pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);

impl<T> AbortOnDrop<T> {
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self(Some(handle))
    }

    /// Await the spawned task while keeping `self` alive across the
    /// await. If the surrounding future is cancelled during the
    /// await, `self` is dropped and our `Drop` impl fires `abort()`.
    /// Compare to a hypothetical `into_inner().await` pattern, which
    /// would release the guard before awaiting — that's the
    /// cancellation-gap bug R34-F2 fixed.
    pub async fn join(mut self) -> std::result::Result<T, tokio::task::JoinError> {
        // Borrow the JoinHandle out of the Option, but DON'T move it
        // out of `self`. `self` lives across this await; if the
        // surrounding future is cancelled here, `self` drops and
        // `Drop::drop` aborts the still-owned handle.
        let handle = self
            .0
            .as_mut()
            .expect("AbortOnDrop already consumed (programming error)");
        let result = handle.await;
        // Task completed (success or panic). Clear the slot so the
        // trailing Drop after this returns is a no-op rather than
        // calling abort() on an already-finished handle.
        self.0 = None;
        result
    }

    /// Request cancellation and await the task while retaining the RAII
    /// guard. Awaiting matters on a multi-thread runtime: `abort()` can race
    /// an already-running poll, so callers that must observe all effects from
    /// that final poll cannot safely abort and continue immediately.
    pub async fn abort_and_join(mut self) -> std::result::Result<T, tokio::task::JoinError> {
        let handle = self
            .0
            .as_mut()
            .expect("AbortOnDrop already consumed (programming error)");
        handle.abort();
        let result = handle.await;
        self.0 = None;
        result
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Regression tests for the `AbortOnDrop` wrapper that bounds
    //! spawned tasks across the workspace (R32-F2, hoisted under
    //! `w4-1`). Without this, dropping the `JoinHandle` would detach
    //! the spawned task — meaning a cancelled parent future (CLI
    //! Ctrl-C, an early `?` return, etc.) couldn't actually stop a
    //! running background task.

    use super::AbortOnDrop;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // Paused virtual time (w4-1 codex review): the relocated test
    // waited only 150ms real time against the task's 500ms natural
    // completion, so it passed whether or not Drop aborted — vacuous
    // since its pull.rs days. Under start_paused the auto-advancing
    // clock deterministically runs a detached task's 500ms sleep
    // BEFORE the test's 700ms wake, so a Drop impl that detaches
    // instead of aborting now fails the assertion, with no wall-clock
    // sensitivity.
    #[tokio::test(start_paused = true)]
    async fn drop_without_consume_aborts_running_task() {
        // The task tries to set the "completed" flag after a delay;
        // wrapping in AbortOnDrop and dropping immediately must
        // prevent the flag from ever being set.
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);

        let guard = AbortOnDrop::new(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            completed_in_task.store(true, Ordering::SeqCst);
        }));
        // Drop the wrapper without awaiting — this is the
        // cancellation path (e.g. the outer future was dropped
        // mid-flight).
        drop(guard);

        // Wait past the task's natural completion point. If abort
        // actually happened the task is dead and the flag never got
        // set; if Drop detached instead, virtual time runs the task's
        // 500ms sleep first and the flag IS set by now.
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert!(
            !completed.load(Ordering::SeqCst),
            "task ran to completion despite AbortOnDrop being dropped"
        );
    }

    #[tokio::test]
    async fn join_returns_value_and_drop_becomes_noop() {
        // Happy path: the caller awaits via `.join()`. The task
        // completes naturally, the value is returned, and the
        // wrapper's Drop is a no-op (slot was cleared inside join).
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);

        let guard = AbortOnDrop::new(tokio::spawn(async move {
            completed_in_task.store(true, Ordering::SeqCst);
            42_u32
        }));

        let value = guard.join().await.expect("task succeeds");
        assert_eq!(value, 42);
        assert!(completed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn abort_and_join_observes_task_cleanup_before_returning() {
        struct Cleanup(Arc<AtomicBool>);

        impl Drop for Cleanup {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let cleaned = Arc::new(AtomicBool::new(false));
        let started = Arc::new(tokio::sync::Notify::new());
        let task_cleaned = Arc::clone(&cleaned);
        let task_started = Arc::clone(&started);
        let guard = AbortOnDrop::new(tokio::spawn(async move {
            let _cleanup = Cleanup(task_cleaned);
            task_started.notify_one();
            std::future::pending::<()>().await;
        }));

        started.notified().await;
        let err = guard
            .abort_and_join()
            .await
            .expect_err("aborted task reports cancellation");
        assert!(err.is_cancelled());
        assert!(
            cleaned.load(Ordering::SeqCst),
            "task cleanup must finish before abort_and_join returns"
        );
    }

    #[tokio::test]
    async fn drop_after_natural_completion_does_not_panic() {
        // If the task happens to complete before Drop fires, the
        // wrapper must still drop cleanly. abort() on a completed
        // JoinHandle is a no-op in tokio; this test pins that
        // expectation in our wrapper.
        let guard = AbortOnDrop::new(tokio::spawn(async {}));
        // Let the task complete.
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(guard);
    }

    // ── R34-F2: cancellation during the join await still aborts ──────

    #[tokio::test]
    async fn cancellation_during_join_await_still_aborts_task() {
        // The load-bearing R34-F2 regression. Pre-fix, the wrapper
        // exposed `into_inner() -> JoinHandle<T>` and callers did
        // `handle.into_inner().await`. That moved the handle out of
        // the wrapper before the await: if the surrounding future was
        // cancelled mid-await, the bare `JoinHandle` was dropped, and
        // tokio detaches on JoinHandle drop. The spawned task kept
        // running.
        //
        // Post-fix, `.join()` holds `self` across the await; if the
        // surrounding future is dropped at that point, `self` drops
        // and `Drop::drop` calls `abort()` on the still-owned handle.
        let completed = Arc::new(AtomicBool::new(false));
        let completed_in_task = Arc::clone(&completed);

        let guard = AbortOnDrop::new(tokio::spawn(async move {
            // Long enough that the test will reliably abort before
            // natural completion.
            tokio::time::sleep(Duration::from_millis(500)).await;
            completed_in_task.store(true, Ordering::SeqCst);
        }));

        // Build the join future and drop it after a short timeout —
        // simulating an outer `tokio::select!` whose other branch
        // fired (the realistic scenario in the daemon's
        // delegated_pull handler when the CLI hangs up).
        let join_fut = guard.join();
        let timed_out = tokio::time::timeout(Duration::from_millis(20), join_fut)
            .await
            .is_err();
        assert!(timed_out, "timeout must fire to drop the join future");

        // Wait well past when the task would have naturally
        // completed. If abort actually fired through the wrapper
        // during the dropped join await, the flag is still false.
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert!(
            !completed.load(Ordering::SeqCst),
            "task ran to completion despite cancellation during join() await — \
             AbortOnDrop is leaking the handle out before the await again"
        );
    }
}
