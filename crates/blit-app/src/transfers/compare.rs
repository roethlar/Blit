//! otp-10b-2: the ONE args→compare mapping for both remote verbs.
//!
//! The old pull client mapped its CLI compare flags onto the wire
//! `ComparisonMode` in `RemotePullClient::build_spec_from_options`; the
//! old push driver ignored every compare flag (R54-F2 documented the
//! silent fall-through to size+mtime). On the unified session the
//! DESTINATION owns the compare decision for BOTH directions, fed by
//! `SessionOpen.compare_mode` — so both verbs map their flags here,
//! with the old pull's precedence, and a push honors `--checksum` /
//! `--size-only` / `--ignore-times` / `--force` exactly like a pull.
//!
//! `--ignore-existing` is the orthogonal "skip if dst exists" axis and
//! rides `SessionOpen.ignore_existing`, not this enum (same split the
//! old wire spec made).

use blit_core::generated::ComparisonMode;

/// The compare-relevant CLI flags, direction-agnostic. Both verb
/// wrappers build this from `&TransferArgs`; the TUI builds it
/// directly (it exposes no compare toggles yet, so all-false).
#[derive(Clone, Copy, Debug, Default)]
pub struct CompareFlags {
    pub checksum: bool,
    pub size_only: bool,
    pub ignore_times: bool,
    pub force: bool,
}

/// Map the user's compare flags onto the wire `ComparisonMode` for a
/// copy/mirror verb — the old pull's exact precedence
/// (`build_spec_from_options`): `--ignore-times` > `--force` >
/// `--size-only` > `--checksum` > the SizeMtime default.
pub fn comparison_mode(flags: CompareFlags) -> ComparisonMode {
    if flags.ignore_times {
        ComparisonMode::IgnoreTimes
    } else if flags.force {
        ComparisonMode::Force
    } else if flags.size_only {
        ComparisonMode::SizeOnly
    } else if flags.checksum {
        ComparisonMode::Checksum
    } else {
        ComparisonMode::SizeMtime
    }
}

/// The move-verb mapping (codex otp-10a F1, mirrored on pull at
/// otp-10b-2): a move deletes the source after the transfer, so every
/// skip must be provably safe. `--checksum` qualifies — a Checksum
/// skip means the destination already holds byte-identical content —
/// so it is honored; everything else maps to `IgnoreTimes` (transfer
/// unconditionally). The metadata-shaped skips (`--size-only`, plain
/// SizeMtime) can skip a changed file and would let the source-delete
/// destroy its only copy; the CLI rejects `--size-only` /
/// `--ignore-times` / `--force` on move up front, and this mapping is
/// safe even if a caller forgets that gate.
pub fn move_comparison_mode(flags: CompareFlags) -> ComparisonMode {
    if flags.checksum {
        ComparisonMode::Checksum
    } else {
        ComparisonMode::IgnoreTimes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(checksum: bool, size_only: bool, ignore_times: bool, force: bool) -> CompareFlags {
        CompareFlags {
            checksum,
            size_only,
            ignore_times,
            force,
        }
    }

    /// The old pull's precedence table, pinned verbatim so the wire
    /// mapping can never drift from what `--checksum` et al. meant on
    /// the old driver (build_spec_from_options).
    #[test]
    fn copy_mapping_matches_the_old_pull_precedence() {
        assert_eq!(
            comparison_mode(flags(false, false, false, false)),
            ComparisonMode::SizeMtime
        );
        assert_eq!(
            comparison_mode(flags(true, false, false, false)),
            ComparisonMode::Checksum
        );
        assert_eq!(
            comparison_mode(flags(false, true, false, false)),
            ComparisonMode::SizeOnly
        );
        assert_eq!(
            comparison_mode(flags(false, false, true, false)),
            ComparisonMode::IgnoreTimes
        );
        assert_eq!(
            comparison_mode(flags(false, false, false, true)),
            ComparisonMode::Force
        );
        // Precedence: ignore-times beats force beats size-only beats
        // checksum (all set → IgnoreTimes; the old driver's if-chain).
        assert_eq!(
            comparison_mode(flags(true, true, true, true)),
            ComparisonMode::IgnoreTimes
        );
        assert_eq!(
            comparison_mode(flags(true, true, false, true)),
            ComparisonMode::Force
        );
        assert_eq!(
            comparison_mode(flags(true, true, false, false)),
            ComparisonMode::SizeOnly
        );
    }

    /// Move never yields a metadata-shaped skip: only a content-proven
    /// Checksum compare or unconditional transfer (otp-10a F1).
    #[test]
    fn move_mapping_is_ignore_times_unless_checksum() {
        assert_eq!(
            move_comparison_mode(flags(false, false, false, false)),
            ComparisonMode::IgnoreTimes
        );
        assert_eq!(
            move_comparison_mode(flags(true, false, false, false)),
            ComparisonMode::Checksum
        );
        // Even flags the CLI rejects upstream stay safe here.
        assert_eq!(
            move_comparison_mode(flags(false, true, true, true)),
            ComparisonMode::IgnoreTimes
        );
    }
}
