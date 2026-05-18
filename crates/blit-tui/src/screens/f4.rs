//! F4 Profile screen — header / records-summary block /
//! predictor coefficients block / footer.
//!
//! Atomic scope for a1-5: read-only display of the
//! profile data already produced by `blit_app::profile`.
//! The design's [c] clear / [d] disable / [e] enable
//! hotkeys + the Verify / Diagnostics sub-blocks land in
//! later A.1 slices.
//!
//! Layout:
//!
//! ```text
//! ┌── header (1 line) ─────────────────────────────┐
//! │ blit-tui · F4 Profile · <state> · <records>    │
//! ├── records summary (Length 4) ──────────────────┤
//! │ Records: N · span: D days · ~Bytes total       │
//! │ Predictor file: <path> (or "(not loaded)")     │
//! ├── predictor block (Min 5) ─────────────────────┤
//! │ copy   α=...  β=...  γ=...                     │
//! │ mirror α=...  β=...  γ=...                     │
//! ├── footer (1 line) ─────────────────────────────┤
//! │ status · q quit · r refresh                    │
//! └────────────────────────────────────────────────┘
//! ```

use crate::diagnostics::{DiagnosticsState, DiagnosticsStatus};
use crate::profile::{ProfileFetchStatus, ProfileState};
use crate::transfer::{TransferState, TransferStatus};
use crate::verify::{VerifyFocus, VerifyState, VerifyStatus};
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
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(5),
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state);
    render_records_summary(frame, chunks[1], state);
    render_predictor(frame, chunks[2], state);
    render_verify(frame, chunks[3], verify, now);
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
            "mirror will DELETE extraneous files at destination · [y/N] to confirm",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        TransferStatus::ConfirmingMove => Line::from(Span::styled(
            "move will DELETE the SOURCE after copy · [y/N] to confirm",
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
        } => Line::from(Span::styled(
            format!(
                "{} done · {} planned · {} copied · {} bytes · {}",
                kind.label(),
                summary.planned_files,
                summary.copied_files,
                summary.total_bytes,
                format_elapsed(finished_at.saturating_duration_since(*started_at)),
            ),
            Style::default().fg(Color::Green),
        )),
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
    let lines = match state.report() {
        Some(report) => summary_lines(report),
        None => vec![Line::from(Span::styled(
            "(no report loaded yet — press r to refresh)",
            Style::default().fg(Color::DarkGray),
        ))],
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

fn render_verify(frame: &mut Frame, area: Rect, verify: &VerifyState, now: Instant) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Verify (local paths only) ");
    let focus = verify.focus();
    let source_line = field_line("Source: ", &verify.source, focus == VerifyFocus::Source);
    let dest_line = field_line(
        "Destin: ",
        &verify.destination,
        focus == VerifyFocus::Destination,
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
    let lines = vec![source_line, dest_line, mode_hint, status_line];
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn field_line(label: &str, value: &str, focused: bool) -> Line<'static> {
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
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
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

fn format_bytes(n: u64) -> String {
    if n >= 1 << 40 {
        format!("{:.2} TiB", n as f64 / (1u64 << 40) as f64)
    } else if n >= 1 << 30 {
        format!("{:.2} GiB", n as f64 / (1u64 << 30) as f64)
    } else if n >= 1 << 20 {
        format!("{:.2} MiB", n as f64 / (1u64 << 20) as f64)
    } else if n >= 1 << 10 {
        format!("{:.2} KiB", n as f64 / (1u64 << 10) as f64)
    } else {
        format!("{n} B")
    }
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
}
