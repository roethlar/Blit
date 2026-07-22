//! F4 Profile screen — header / records-summary block /
//! predictor coefficients block / footer.
//!
//! Renders the profile data from `blit_app::profile`, the
//! Verify form, the Diagnostics-dump block, and the local
//! Transfer block. The lifecycle hotkeys are wired: [d]/[e]
//! toggle history recording and [c] clears the log behind a
//! y/N confirm (d-66) — the records-summary block shows the
//! red confirm prompt while it's armed.
//!
//! Layout:
//!
//! ```text
//! ┌── header (1 line) ─────────────────────────────┐
//! │ blit-tui · F4 Profile · <state> · <records>    │
//! ├── records summary (Length 4) ──────────────────┤
//! │ Records: N · span: D days · ~Bytes total       │
//! │ (or the red d-66 clear-confirm prompt)         │
//! ├── predictor block (Min 2) ─────────────────────┤
//! │ copy   α=...  β=...  γ=...                     │
//! │ mirror α=...  β=...  γ=...                     │
//! ├── verify form (Length 9) ──────────────────────┤
//! ├── diagnostics (Length 3) ──────────────────────┤
//! ├── local transfer (Length 3) ───────────────────┤
//! ├── footer (1 line) ─────────────────────────────┤
//! │ status · q quit · r refresh · c/d/e · s · tab  │
//! └────────────────────────────────────────────────┘
//! ```

use crate::diagnostics::{DiagnosticsState, DiagnosticsStatus};
use crate::profile::{ProfileFetchStatus, ProfileState};
use crate::transfer::{TransferState, TransferStatus};
use crate::verify::{VerifyFocus, VerifyState, VerifyStatus};
use blit_app::display::{format_bps, format_bytes};
use blit_app::profile::{PredictorReport, ProfileReport, ProfileSummary};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::time::{Duration, Instant};

/// d-8: format an elapsed Duration as a compact
/// human-readable string. Sub-second → "Nms", 1-60s →
/// "N.Ns", 1-60m → "Nm Ns", longer → "Nh Nm". The Done
/// banners surface this so the operator can see "copy
/// took 12.3s" or "compare took 432ms" without a
/// stopwatch.
fn format_elapsed(d: Duration) -> String {
    let total_ms = d.as_millis();
    if total_ms < 1000 {
        return format!("{}ms", total_ms);
    }
    let total_secs = d.as_secs();
    if total_secs < 60 {
        // tenths of a second precision for under a minute
        let tenths = (d.as_millis() % 1000) / 100;
        return format!("{}.{}s", total_secs, tenths);
    }
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins < 60 {
        return format!("{}m {}s", mins, secs);
    }
    let hours = mins / 60;
    let mins = mins % 60;
    format!("{}h {}m", hours, mins)
}

#[cfg(test)]
mod elapsed_tests {
    use super::*;

    #[test]
    fn format_elapsed_milliseconds() {
        assert_eq!(format_elapsed(Duration::from_millis(0)), "0ms");
        assert_eq!(format_elapsed(Duration::from_millis(432)), "432ms");
        assert_eq!(format_elapsed(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn format_elapsed_seconds_with_tenths() {
        assert_eq!(format_elapsed(Duration::from_millis(1000)), "1.0s");
        assert_eq!(format_elapsed(Duration::from_millis(12340)), "12.3s");
        assert_eq!(format_elapsed(Duration::from_millis(59900)), "59.9s");
    }

    #[test]
    fn format_elapsed_minutes_seconds() {
        assert_eq!(format_elapsed(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_elapsed(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_elapsed(Duration::from_secs(59 * 60 + 59)), "59m 59s");
    }

    #[test]
    fn format_elapsed_hours_minutes() {
        assert_eq!(format_elapsed(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_elapsed(Duration::from_secs(3600 + 1800)), "1h 30m");
    }
}

/// d-10: format an effective transfer rate as a compact
/// human-readable string. Returns `None` when the rate
/// is meaningless (zero bytes, sub-millisecond duration,
/// or rate below 1 B/s after rounding) so the caller can
/// suppress the trailing rate fragment instead of showing
/// a misleading "0 B/s" for a 1-file copy that completed
/// instantly.
///
/// The shared presenter formatter owns binary units and
/// precision so the Done banner matches every other rate.
fn format_rate(bytes: u64, duration: Duration) -> Option<String> {
    if bytes == 0 {
        return None;
    }
    let ms = duration.as_millis();
    if ms == 0 {
        return None;
    }
    let bytes_per_sec = (bytes as u128)
        .saturating_mul(1000)
        .checked_div(ms)
        .unwrap_or(0)
        .min(u64::MAX as u128) as u64;
    if bytes_per_sec == 0 {
        return None;
    }
    Some(format_bps(bytes_per_sec))
}

#[cfg(test)]
mod rate_tests {
    use super::*;

    #[test]
    fn format_rate_returns_none_for_zero_bytes() {
        assert!(format_rate(0, Duration::from_secs(1)).is_none());
    }

    #[test]
    fn format_rate_returns_none_for_zero_duration() {
        // A copy that "completed instantly" by Instant
        // resolution has no meaningful rate.
        assert!(format_rate(1024, Duration::from_millis(0)).is_none());
    }

    #[test]
    fn format_rate_bytes_per_second() {
        // 512 bytes in 1s = 512 B/s — below KiB cutoff.
        let s = format_rate(512, Duration::from_secs(1)).expect("rate");
        assert_eq!(s, "512 B/s");
    }

    #[test]
    fn format_rate_kibibytes_per_second() {
        // 1 KiB in 1s = 1.00 KiB/s.
        let s = format_rate(1024, Duration::from_secs(1)).expect("rate");
        assert_eq!(s, "1.00 KiB/s");
    }

    #[test]
    fn format_rate_mebibytes_per_second() {
        // 100 MiB in 10s = 10 MiB/s.
        let s = format_rate(100 * 1024 * 1024, Duration::from_secs(10)).expect("rate");
        assert_eq!(s, "10.00 MiB/s");
    }

    #[test]
    fn format_rate_gibibytes_per_second() {
        // 2 GiB in 1s = 2 GiB/s. (Hypothetical for unit
        // testing; F4 doesn't normally see these speeds.)
        let s = format_rate(2 * 1024 * 1024 * 1024, Duration::from_secs(1)).expect("rate");
        assert_eq!(s, "2.00 GiB/s");
    }
}

/// Render the F4 pane into a caller-supplied area (router-aware).
///
/// d-2 adds the Verify block underneath the predictor
/// section. When the operator hits `Tab` the focus walks
/// into the Source / Destination fields; chars + Backspace
/// edit, Enter triggers the run.
pub fn render_into(
    frame: &mut Frame,
    area: Rect,
    state: &ProfileState,
    verify: &VerifyState,
    diagnostics: &DiagnosticsState,
    transfer: &TransferState,
    now: Instant,
    // e-10: `[theme] accent_color` for the focused Verify-field
    // highlight (matches the tab strip / F2 / F3).
    accent: Color,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            // d-17 trades 3 lines from the predictor block
            // for verify's preview block (first differing
            // path / first missing path / first error).
            // Min(2) still keeps the predictor visible —
            // even a single-line predictor message
            // ("not loaded") fits.
            Constraint::Min(2),
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state);
    render_records_summary(frame, chunks[1], state);
    render_predictor(frame, chunks[2], state);
    render_verify(frame, chunks[3], verify, now, accent);
    render_diagnostics(frame, chunks[4], diagnostics);
    render_transfer(frame, chunks[5], transfer, now);
    render_footer(frame, chunks[6], state.status(), verify, now);
}

fn render_transfer(frame: &mut Frame, area: Rect, transfer: &TransferState, now: Instant) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Local transfer (C copy · M mirror · V move) ");
    let line = match transfer.status() {
        TransferStatus::Idle => Line::from(Span::styled(
            "press `C` to copy · `M` to mirror · `V` to move Source → Destination",
            Style::default().fg(Color::DarkGray),
        )),
        TransferStatus::ConfirmingMirror => Line::from(Span::styled(
            "mirror will DELETE extraneous files at destination · [y / N or Esc]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        TransferStatus::ConfirmingMove => Line::from(Span::styled(
            "move will DELETE the SOURCE after copy · [y / N or Esc]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        TransferStatus::Running { kind, started_at } => Line::from(Span::styled(
            format!(
                "{} running... ({})",
                kind.label(),
                format_elapsed(now.saturating_duration_since(*started_at))
            ),
            Style::default().fg(Color::Yellow),
        )),
        TransferStatus::Done {
            kind,
            summary,
            started_at,
            finished_at,
        } => {
            let elapsed = finished_at.saturating_duration_since(*started_at);
            let mut line = format!(
                "{} done · {} planned · {} copied · {} bytes · {}",
                kind.label(),
                summary.planned_files,
                summary.copied_files,
                summary.total_bytes,
                format_elapsed(elapsed),
            );
            // d-10: append effective throughput when it's
            // meaningful (suppressed for 0-byte / instant
            // copies — see `format_rate`).
            if let Some(rate) = format_rate(summary.total_bytes, elapsed) {
                line.push_str(" · ");
                line.push_str(&rate);
            }
            Line::from(Span::styled(line, Style::default().fg(Color::Green)))
        }
        TransferStatus::Error { kind, message } => Line::from(Span::styled(
            format!("{} failed: {message}", kind.label()),
            Style::default().fg(Color::Red),
        )),
    };
    let para = Paragraph::new(vec![line]).block(block);
    frame.render_widget(para, area);
}

fn render_diagnostics(frame: &mut Frame, area: Rect, diagnostics: &DiagnosticsState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Diagnostics ");
    let line = match diagnostics.status() {
        DiagnosticsStatus::Idle => Line::from(Span::styled(
            "press `s` to dump a snapshot of the Verify Source → Destination pair",
            Style::default().fg(Color::DarkGray),
        )),
        DiagnosticsStatus::Running => Line::from(Span::styled(
            "writing diagnostics snapshot...",
            Style::default().fg(Color::Yellow),
        )),
        DiagnosticsStatus::Done { path, .. } => Line::from(Span::styled(
            format!("wrote {}", path.display()),
            Style::default().fg(Color::Green),
        )),
        DiagnosticsStatus::Error { message } => Line::from(Span::styled(
            format!("error: {message}"),
            Style::default().fg(Color::Red),
        )),
    };
    let para = Paragraph::new(vec![line]).block(block);
    frame.render_widget(para, area);
}

fn render_header(frame: &mut Frame, area: Rect, state: &ProfileState) {
    let summary = state
        .report()
        .map(|r| {
            if r.enabled {
                format!("history enabled · {} records", r.records.len())
            } else {
                "history disabled".to_string()
            }
        })
        .unwrap_or_else(|| "not loaded".to_string());
    let title = format!(" blit-tui · F4 Profile · {summary} ");
    let para = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(para, area);
}

fn render_records_summary(frame: &mut Frame, area: Rect, state: &ProfileState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Profile (local performance history) ");
    // d-66: while a `[c] clear` confirm is armed, the block
    // shows the destructive prompt instead of the summary —
    // mirroring how the Local-transfer block surfaces its
    // mirror/move confirms (red banner, no footer swap).
    let lines = if state.is_confirming_clear() {
        vec![Line::from(Span::styled(
            "clear ALL local performance history? this is permanent · [y / N or Esc]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))]
    } else {
        match state.report() {
            Some(report) => summary_lines(report),
            None => vec![Line::from(Span::styled(
                "(no report loaded yet — press r to refresh)",
                Style::default().fg(Color::DarkGray),
            ))],
        }
    };
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_predictor(frame: &mut Frame, area: Rect, state: &ProfileState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Predictor coefficients ");
    let lines = match state.report().and_then(|r| r.predictor.as_ref()) {
        Some(predictor) => predictor_lines(predictor),
        None => vec![
            Line::from(Span::styled(
                "(predictor file not loaded)",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Run some transfers to populate the predictor — see `blit profile`.",
                Style::default().fg(Color::DarkGray),
            )),
        ],
    };
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_verify(frame: &mut Frame, area: Rect, verify: &VerifyState, now: Instant, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Verify (local paths only) ");
    let focus = verify.focus();
    let source_line = field_line(
        "Source: ",
        &verify.source,
        focus == VerifyFocus::Source,
        accent,
    );
    let dest_line = field_line(
        "Destin: ",
        &verify.destination,
        focus == VerifyFocus::Destination,
        accent,
    );
    // Compose a single mode line that surfaces both d-6
    // (size+mtime vs checksum) and d-7 (two-way vs
    // one-way) toggles. Magenta whenever EITHER non-default
    // is on, so the operator's eye catches the deviation.
    let checksum_label = if verify.use_checksum() {
        "checksum"
    } else {
        "size+mtime"
    };
    let direction_label = if verify.one_way() {
        "one-way"
    } else {
        "two-way"
    };
    let mode_text = format!(
        "Mode: {checksum_label} · {direction_label} · H toggles hash · O toggles direction"
    );
    let mode_style = if verify.use_checksum() || verify.one_way() {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mode_hint = Line::from(Span::styled(mode_text, mode_style));
    let status_line = match verify.status() {
        VerifyStatus::Idle => Line::from(Span::styled(
            "tab: enter editing · enter: run · esc: leave editing",
            Style::default().fg(Color::DarkGray),
        )),
        VerifyStatus::Running { started_at } => Line::from(Span::styled(
            format!(
                "running compare_trees... ({})",
                format_elapsed(now.saturating_duration_since(*started_at))
            ),
            Style::default().fg(Color::Yellow),
        )),
        VerifyStatus::Done {
            result,
            started_at,
            finished_at,
        } => Line::from(Span::styled(
            format!(
                "matches: {} · differ: {} · missing-on-src: {} · missing-on-dst: {} · errors: {} · {}",
                result.matching,
                result.differing.len(),
                result.missing_on_src.len(),
                result.missing_on_dest.len(),
                result.errors.len(),
                format_elapsed(finished_at.saturating_duration_since(*started_at)),
            ),
            Style::default().fg(Color::Green),
        )),
        VerifyStatus::Error { message } => Line::from(Span::styled(
            format!("error: {message}"),
            Style::default().fg(Color::Red),
        )),
    };
    let mut lines = vec![source_line, dest_line, mode_hint, status_line];
    // d-17: on Done, show the first entry from each
    // non-empty category (differ / missing-on-dst /
    // missing-on-src / errors). Operator sees the
    // headline count AND the first concrete offender,
    // so debugging a mismatch doesn't always require a
    // diagnostics snapshot.
    if let VerifyStatus::Done { result, .. } = verify.status() {
        for line in verify_preview_lines(result, 3) {
            lines.push(line);
        }
    }
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

/// d-17: produce up to `max` "first entry" preview lines
/// from a `CheckResult`. Order: differing → missing-on-dst
/// → missing-on-src → errors, so the most actionable
/// category (an existing file that differs) comes first.
/// Empty categories are skipped — a clean compare with
/// only matches returns an empty Vec.
fn verify_preview_lines(result: &blit_app::check::CheckResult, max: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let warn = Style::default().fg(Color::Yellow);
    let err = Style::default().fg(Color::Red);
    if !result.differing.is_empty() && lines.len() < max {
        let d = &result.differing[0];
        lines.push(Line::from(Span::styled(
            format!("  differ[0]: {} — {}", d.path, d.reason),
            warn,
        )));
    }
    if !result.missing_on_dest.is_empty() && lines.len() < max {
        lines.push(Line::from(Span::styled(
            format!("  missing-on-dst[0]: {}", result.missing_on_dest[0]),
            warn,
        )));
    }
    if !result.missing_on_src.is_empty() && lines.len() < max {
        lines.push(Line::from(Span::styled(
            format!("  missing-on-src[0]: {}", result.missing_on_src[0]),
            warn,
        )));
    }
    if !result.errors.is_empty() && lines.len() < max {
        let (path, msg) = &result.errors[0];
        lines.push(Line::from(Span::styled(
            format!("  errors[0]: {} — {}", path, msg),
            err,
        )));
    }
    lines
}

fn field_line(label: &str, value: &str, focused: bool, accent: Color) -> Line<'static> {
    let label_span = Span::styled(
        label.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    );
    let value_text = if focused {
        // Render with an explicit cursor caret so the
        // operator sees where typing lands. Using a
        // closing pipe avoids cursor-on-empty-line
        // ambiguity that would happen with a bare block
        // cursor.
        format!("{value}▏")
    } else {
        if value.is_empty() {
            "(empty)".to_string()
        } else {
            value.to_string()
        }
    };
    let value_style = if focused {
        // e-10: the focused field highlight honors the `[theme]
        // accent_color` with a contrasting foreground (white on dark
        // accents), matching the tab strip (e-7) / F2 (e-9) / F3 (e-10).
        Style::default()
            .fg(super::contrasting_fg(accent))
            .bg(accent)
            .add_modifier(Modifier::BOLD)
    } else if value.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    Line::from(vec![label_span, Span::styled(value_text, value_style)])
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    status: &ProfileFetchStatus,
    verify: &VerifyState,
    now: Instant,
) {
    let status_span = match status {
        ProfileFetchStatus::Idle => {
            Span::styled("idle · press r", Style::default().fg(Color::DarkGray))
        }
        ProfileFetchStatus::Pending => {
            Span::styled("reading...", Style::default().fg(Color::Yellow))
        }
        ProfileFetchStatus::Loaded { fetched_at } => Span::styled(
            format!("loaded · {}", format_since(now, *fetched_at)),
            Style::default().fg(Color::Green),
        ),
        ProfileFetchStatus::Error { message } => {
            Span::styled(format!("error: {message}"), Style::default().fg(Color::Red))
        }
    };
    // When the Verify form has focus, the footer hint
    // line swaps to editing keys (Tab/Enter/Esc) since
    // c/d/e/r are then being typed as text into the
    // fields rather than acting as profile-lifecycle
    // shortcuts.
    let spans: Vec<Span<'static>> = if verify.focus().is_editing() {
        vec![
            status_span,
            Span::raw("  ·  editing form  ·  "),
            Span::styled("tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" next field  ·  "),
            Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" run  ·  "),
            Span::styled("esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" leave"),
        ]
    } else {
        vec![
            status_span,
            Span::raw("  ·  "),
            Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" quit  ·  "),
            Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" refresh  ·  "),
            Span::styled("c", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" clear  ·  "),
            Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" disable  ·  "),
            Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" enable  ·  "),
            Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" snapshot  ·  "),
            Span::styled("tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" verify"),
        ]
    };
    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

fn summary_lines(report: &ProfileReport) -> Vec<Line<'static>> {
    let count = report.records.len();
    let span_days = span_days(&report.records);
    let total_bytes: u64 = report.records.iter().map(|r| r.total_bytes).sum();
    let head = format!(
        "Records: {} · span: {} days · ~{} total",
        count,
        span_days,
        format_bytes(total_bytes),
    );
    let predictor_path = match &report.predictor_path {
        Some(p) => format!("Predictor file: {}", p.display()),
        None => "Predictor file: (not loaded)".to_string(),
    };
    let enabled_line = if report.enabled {
        Span::styled(
            "History recording: enabled",
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled(
            "History recording: disabled (no new records being captured)",
            Style::default().fg(Color::Yellow),
        )
    };
    vec![
        Line::from(head),
        Line::from(predictor_path),
        Line::from(enabled_line),
    ]
}

fn predictor_lines(predictor: &PredictorReport) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    extend_predictor_block(&mut lines, "copy", &predictor.copy);
    extend_predictor_block(&mut lines, "mirror", &predictor.mirror);
    lines
}

fn extend_predictor_block(lines: &mut Vec<Line<'static>>, label: &str, summary: &ProfileSummary) {
    match &summary.coefficients {
        Some(c) => {
            lines.push(Line::from(Span::styled(
                format!(
                    "[{label}]  n={}  fallback={}",
                    summary.observations, summary.fallback_depth
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!(
                "  planner  α={:.2} ms/file  β={:.2} ms/MiB  γ={:.2} ms",
                c.planner.alpha_ms_per_file, c.planner.beta_ms_per_mb, c.planner.gamma_ms,
            )));
            lines.push(Line::from(format!(
                "  transfer α={:.2} ms/file  β={:.2} ms/MiB  γ={:.2} ms",
                c.transfer.alpha_ms_per_file, c.transfer.beta_ms_per_mb, c.transfer.gamma_ms,
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                format!("[{label}]  (no profile yet — needs ≥5 observations)"),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
}

fn span_days(records: &[blit_core::perf_history::PerformanceRecord]) -> u64 {
    if records.is_empty() {
        return 0;
    }
    let mut min = u128::MAX;
    let mut max = 0u128;
    for r in records {
        if r.timestamp_epoch_ms < min {
            min = r.timestamp_epoch_ms;
        }
        if r.timestamp_epoch_ms > max {
            max = r.timestamp_epoch_ms;
        }
    }
    if max <= min {
        return 0;
    }
    let span_ms = max - min;
    (span_ms / (24 * 60 * 60 * 1000)) as u64
}

fn format_since(now: Instant, then: Instant) -> String {
    let elapsed = now.saturating_duration_since(then);
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_app::profile::{DurationCoefficients, PredictorReport, ProfileReport, ProfileSummary};
    use blit_core::perf_history::{
        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
    };
    use blit_core::perf_predictor::PredictorCoefficients;

    fn record(epoch_ms: u128, bytes: u64) -> PerformanceRecord {
        PerformanceRecord {
            schema_version: 2,
            timestamp_epoch_ms: epoch_ms,
            mode: TransferMode::Copy,
            run_kind: RunKind::default(),
            source_fs: None,
            dest_fs: None,
            file_count: 0,
            total_bytes: bytes,
            options: OptionSnapshot {
                dry_run: false,
                preserve_symlinks: false,
                include_symlinks: false,
                skip_unchanged: false,
                checksum: false,
                compare_mode: CompareModeSnapshot::default(),
                workers: 1,
            },
            fast_path: None,
            planner_duration_ms: 0,
            transfer_duration_ms: 0,
            stall_events: 0,
            error_count: 0,
            tar_shard_tasks: 0,
            tar_shard_files: 0,
            tar_shard_bytes: 0,
            raw_bundle_tasks: 0,
            raw_bundle_files: 0,
            raw_bundle_bytes: 0,
            large_tasks: 0,
            large_bytes: 0,
        }
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn format_bytes_picks_correct_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1u64 << 20), "1.00 MiB");
        assert_eq!(format_bytes(1u64 << 30), "1.00 GiB");
        assert_eq!(format_bytes(1u64 << 40), "1.00 TiB");
    }

    /// e-10: a focused Verify field highlights with the themed accent
    /// background + a contrasting foreground (white on dark accents,
    /// black on light); an unfocused field is not themed.
    #[test]
    fn verify_focused_field_uses_accent_with_contrast() {
        // Dark accent → contrasting white foreground.
        let line = field_line("Source: ", "x", true, Color::Red);
        assert_eq!(line.spans[1].style.bg, Some(Color::Red));
        assert_eq!(
            line.spans[1].style.fg,
            Some(Color::White),
            "dark accent → white fg"
        );
        // Light accent → black foreground.
        let line = field_line("Source: ", "x", true, Color::Cyan);
        assert_eq!(line.spans[1].style.bg, Some(Color::Cyan));
        assert_eq!(
            line.spans[1].style.fg,
            Some(Color::Black),
            "light accent → black fg"
        );
        // Unfocused field carries no accent background.
        let line = field_line("Source: ", "x", false, Color::Red);
        assert_ne!(line.spans[1].style.bg, Some(Color::Red));
    }

    #[test]
    fn span_days_computes_from_timestamps() {
        // Two records 3 days apart.
        let day_ms: u128 = 24 * 60 * 60 * 1000;
        let records = vec![record(1_000, 0), record(1_000 + 3 * day_ms, 0)];
        assert_eq!(span_days(&records), 3);
    }

    #[test]
    fn span_days_zero_for_empty_or_single_record() {
        assert_eq!(span_days(&[]), 0);
        assert_eq!(span_days(&[record(1_000, 0)]), 0);
    }

    #[test]
    fn summary_lines_total_bytes() {
        let report = ProfileReport {
            enabled: true,
            records: vec![record(1, 1u64 << 30), record(2, 2u64 << 30)],
            predictor_path: None,
            predictor: None,
        };
        let lines = summary_lines(&report);
        let head = line_text(&lines[0]);
        assert!(head.contains("Records: 2"));
        // 1+2 = 3 GiB.
        assert!(head.contains("3.00 GiB"));
    }

    #[test]
    fn predictor_lines_renders_coefficients_and_observations() {
        let coeffs = PredictorCoefficients {
            alpha_ms_per_file: 12.4,
            beta_ms_per_mb: 0.31,
            gamma_ms: 18.0,
        };
        let predictor = PredictorReport {
            copy: ProfileSummary {
                coefficients: Some(DurationCoefficients {
                    planner: coeffs.clone(),
                    transfer: coeffs,
                }),
                observations: 42,
                fallback_depth: 1,
            },
            mirror: ProfileSummary {
                coefficients: None,
                observations: 0,
                fallback_depth: 0,
            },
        };
        let lines = predictor_lines(&predictor);
        // copy block: header + planner + transfer = 3 lines;
        // mirror block: 1 line. 4 total.
        assert_eq!(lines.len(), 4);
        let header = line_text(&lines[0]);
        assert!(header.contains("[copy]"));
        assert!(header.contains("n=42"));
        let planner = line_text(&lines[1]);
        assert!(planner.contains("planner"));
        assert!(planner.contains("12.40"));
        assert!(planner.contains("0.31"));
        assert!(planner.contains("18.00"));
        let mirror = line_text(&lines[3]);
        assert!(mirror.contains("[mirror]"));
        assert!(mirror.contains("no profile yet"));
    }

    #[test]
    fn summary_lines_renders_disabled_warning() {
        let report = ProfileReport {
            enabled: false,
            records: Vec::new(),
            predictor_path: None,
            predictor: None,
        };
        let lines = summary_lines(&report);
        let recording_line = line_text(&lines[2]);
        assert!(recording_line.contains("disabled"));
    }

    // d-17: verify-result preview lines.

    fn empty_check_result() -> blit_app::check::CheckResult {
        blit_app::check::CheckResult::default()
    }

    #[test]
    fn verify_preview_empty_result_returns_no_lines() {
        // A clean compare (all matching, no diffs) needs
        // no preview block — the headline status line
        // already says "matches: N · differ: 0 · ...".
        let lines = verify_preview_lines(&empty_check_result(), 3);
        assert!(lines.is_empty());
    }

    #[test]
    fn verify_preview_shows_first_differ_first() {
        let mut result = empty_check_result();
        result.differing.push(blit_app::check::DiffEntry {
            path: "src/a.txt".to_string(),
            reason: "size 1024 vs 2048".to_string(),
            src_size: 1024,
            dst_size: 2048,
        });
        result.missing_on_dest.push("src/b.txt".to_string());
        let lines = verify_preview_lines(&result, 3);
        assert_eq!(lines.len(), 2);
        let first = line_text(&lines[0]);
        assert!(first.contains("differ[0]"));
        assert!(first.contains("src/a.txt"));
        assert!(first.contains("size 1024 vs 2048"));
        let second = line_text(&lines[1]);
        assert!(second.contains("missing-on-dst[0]"));
        assert!(second.contains("src/b.txt"));
    }

    #[test]
    fn verify_preview_caps_at_max_even_with_all_categories() {
        let mut result = empty_check_result();
        result.differing.push(blit_app::check::DiffEntry {
            path: "a".to_string(),
            reason: "r".to_string(),
            src_size: 0,
            dst_size: 0,
        });
        result.missing_on_dest.push("b".to_string());
        result.missing_on_src.push("c".to_string());
        result.errors.push(("d".to_string(), "msg".to_string()));
        // 4 non-empty categories but max=2.
        let lines = verify_preview_lines(&result, 2);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn verify_preview_shows_errors_in_red() {
        let mut result = empty_check_result();
        result
            .errors
            .push(("src/oops".to_string(), "permission denied".to_string()));
        let lines = verify_preview_lines(&result, 3);
        assert_eq!(lines.len(), 1);
        let text = line_text(&lines[0]);
        assert!(text.contains("errors[0]"));
        assert!(text.contains("src/oops"));
        assert!(text.contains("permission denied"));
    }
}

#[cfg(test)]
mod render_tests {
    //! audit-6 item 2: the existing F4 tests cover the pure line-builders
    //! (predictor/summary/verify-preview), but never drive the actual
    //! `render_into` through a real ratatui backend. These render the
    //! whole F4 pane (Profile + Verify + Diagnostics + Transfer) into a
    //! `TestBackend` and assert it doesn't panic — covering layout
    //! arithmetic and widget construction, including the small-area clamp.
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn draw_f4_at(w: u16, h: u16) {
        let state = ProfileState::default();
        let verify = VerifyState::default();
        let diagnostics = DiagnosticsState::default();
        let transfer = TransferState::default();
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_into(
                    frame,
                    frame.area(),
                    &state,
                    &verify,
                    &diagnostics,
                    &transfer,
                    Instant::now(),
                    Color::Cyan,
                );
            })
            .expect("draw must not error");
    }

    #[test]
    fn f4_renders_default_state_without_panic() {
        draw_f4_at(120, 40);
    }

    #[test]
    fn f4_renders_tiny_area_without_panic() {
        // F4's fixed-height layout wants ~23 rows; a far smaller terminal
        // must clamp gracefully rather than panic on a zero/negative span.
        draw_f4_at(8, 3);
    }
}
