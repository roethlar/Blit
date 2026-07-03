use blit_core::perf_history;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub perf_history_enabled: bool,
}

impl AppContext {
    pub fn load() -> Self {
        let perf_history_enabled = match perf_history::perf_history_enabled() {
            Ok(enabled) => enabled,
            Err(err) => {
                eprintln!(
                    "[warn] failed to read performance history settings (defaulting to enabled): {err:?}"
                );
                true
            }
        };
        Self {
            perf_history_enabled,
        }
    }
}
