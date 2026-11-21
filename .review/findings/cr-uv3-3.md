# cr-uv3-3: A utility path was read by the receiving system's own path rules

**Severity**: HIGH — on Unix a name the commands print is refused; on Windows a name is read as two and another entry can be removed
**Status**: Verified — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`, which required no change to it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c99ad483`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-5.

## Evidence
"On Linux, `find` printed the module-root entries `\notes` and `C:receipt`, then `du` rejected both printed operands as Windows paths. On Windows, source inspection shows the converse risk. The newly accepted `quarter\notes.pst` becomes two native components, so `rm` can target `quarter/notes.pst`. Decode utility paths as `/`-separated components, preserve legal Unix names, and reject components Windows cannot represent before filesystem access."

Cause. cr-uv2-5 read the DAEMON'S part of an operand apart from its path on the asking side. On the daemon the path still went to `resolve_relative_path`: a check made for transfers between unlike systems, which refuses anything that looks like a Windows path on any system, and then the system's own path reader, which on Windows splits at `\`.

## Intake
Admitted as the reviewer put it. The Windows half is from the source.

## What
`path_safety::typed_components`: a typed path is its components between `/`, each ONE name whatever bytes it holds; empty components and `.` are passed over, `..` and a NUL refused. Under the Windows rule a component that is not one name there — it holds a `\` or a `:`, is not text, or reads as a drive, a device or two names — is refused (`… cannot be the name of an entry on this system`) before the filesystem is asked anything. The daemon reads every utility path with it (`resolve_typed`), text or bytes.

## Guard proof
- `rm.rs::a_name_another_system_reads_as_a_path_is_a_name_here` (Unix), through the binary: `\notes`, `C:receipt`, `a:b\c` at the module's root and `\notes`, `D:\x` below it — each printed by `find`, the line given to `du`, `df` and `rm`, which removes that file and no other.
- `path_safety::…::a_typed_path_is_its_components_between_slashes`: eleven paths and their component counts, the bytes of a name kept; `..` and NUL refused under both rules; under the Windows rule seven paths, one name or refused in those words.
- Mutations, each restored byte for byte: the daemon reading a typed path by the transfers' check, as before → the end-to-end guard; a `\` or `:` let through under the Windows rule → the unit guard; a `..` passed over like a `.` → the unit guard.

## Files changed
- `crates/blit-core/src/path_safety.rs` — `typed_components`; guard.
- `crates/blit-daemon/src/service/admin.rs` — `resolve_typed`.
- `crates/blit-cli/tests/rm.rs` — guard.

## Known gaps
- **Nothing ran on Windows.** The Windows rule is checked on Linux as far as its text goes (`\`, `:`, not text); that a reserved device name or a trailing dot is refused rests on the system's own path reader, which only Windows has.
- A TRANSFER's paths are not changed: they keep the check made for unlike systems.
- Run on Linux only.
