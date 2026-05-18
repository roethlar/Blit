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

use crate::profile::{ProfileFetchStatus, ProfileState};
use blit_app::profile::{PredictorReport, ProfileReport, ProfileSummary};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::time::Instant;

pub fn render(frame: &mut Frame, state: &ProfileState, now: Instant) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], state);
    render_records_summary(frame, chunks[1], state);
    render_predictor(frame, chunks[2], state);
    render_footer(frame, chunks[3], state.status(), now);
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

fn render_footer(frame: &mut Frame, area: Rect, status: &ProfileFetchStatus, now: Instant) {
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
    let line = Line::from(vec![
        status_span,
        Span::raw("  ·  "),
        Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  ·  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" refresh"),
    ]);
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
