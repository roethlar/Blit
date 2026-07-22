//! Human-readable string conversions shared by every presenter
//! (CLI text output, TUI panes, JSON-embedded reason strings).
//!
//! Moved from `crates/blit-cli/src/util.rs` as part of the
//! Phase 5 A.0 split. `format_bytes` is also used inside
//! `blit_app::check::compare_trees` to populate
//! `DiffEntry.reason` strings; pre-A.0 a private duplicate
//! lived in the check module to avoid the cross-crate
//! dependency before util.rs was split — now consolidated.

/// Render a byte count in human-readable form (B / KiB / MiB /
/// GiB / TiB, two decimal places past KiB). Always uses binary
/// (1024-based) units. Returns `"0 B"` for zero.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.2} {}", UNITS[unit])
}

/// Render bytes per second with binary units and the same precision as
/// [`format_bytes`]. The suffix is included in the returned string.
pub fn format_bps(bytes_per_second: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_second))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_b() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn rolls_up_to_kib_then_mib_then_gib() {
        assert!(format_bytes(2048).ends_with("KiB"));
        assert!(format_bytes(2 * 1024 * 1024).ends_with("MiB"));
        assert!(format_bytes(2 * 1024 * 1024 * 1024).ends_with("GiB"));
    }

    #[test]
    fn caps_at_tib() {
        // 8 PiB still renders in TiB — the formatter doesn't
        // know about PiB / EiB.
        assert!(format_bytes(8 * 1024u64.pow(5)).ends_with("TiB"));
    }

    #[test]
    fn bps_uses_binary_boundaries_and_includes_suffix() {
        assert_eq!(format_bps(0), "0 B/s");
        assert_eq!(format_bps(1023), "1023 B/s");
        assert_eq!(format_bps(1024), "1.00 KiB/s");
        assert_eq!(format_bps(1 << 20), "1.00 MiB/s");
        assert_eq!(format_bps(1 << 30), "1.00 GiB/s");
        assert_eq!(format_bps(1 << 40), "1.00 TiB/s");
    }
}
