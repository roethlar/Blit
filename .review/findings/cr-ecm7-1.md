# cr-ecm7-1: A `--files-from` name went on the wire with the sender's separators

**Severity**: HIGH — a list read on Windows reached a Linux peer naming other files than it named; under the name-list deletion rule the peer selected an unrelated file for deletion
**Status**: Fixed — awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm7-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r7` over `3bc3ee47..cbab0c39` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r7.codex.md`.

## Evidence
A Windows `--files-from` entry `sub\name.txt` names a nested file. The sender kept the native separator. The reviewer replayed that wire value through the Linux deletion planner with `sub/name.txt` in the source manifest: it selected the unrelated destination file literally named `sub\name.txt` for deletion; sending `sub/name.txt` kept it. The encoding predates this plan, but the plan's deletion rule (cr-ecm5-5, cr-ecm6-3) now relies on it to say exactly what was listed. Asked for: the existing component-wise POSIX encoder, components validated on receipt, and the correction versioned for mixed peers.

Cause: `build_spec` sent each listed name as `to_string_lossy()` of the path the list line made on the sending host, and `filter_from_spec` made a path of that text on the receiving one. Two hosts that write paths differently read one text as two different paths. (A retry's list already went through the component-wise encoder; the list a command is given did not.)

## Predicted observable failure
A list read on Windows, any route to a Linux or macOS peer: no listed file in a folder is found there; on a mirror, a file whose own name is the listed text is taken for the one listed. The other way round, a Linux name holding a `\` was read on Windows as a folder and a file.

## Intake
Admitted, with all three parts the reviewer asked for.

## What
- **Sent by component.** A listed name travels as its components joined by `/` (`path_posix::relative_path_to_posix`, the encoder manifests use), sorted, each once.
- **Read by component.** On receipt each piece between two `/` is ONE component whatever it holds; it is never handed to the receiving host's path parser to split again (`operation_spec::listed_name`). A piece that is no name at all — empty, `.`, `..`, holding a NUL — refuses the list: no sender of this contract writes one. A piece that is a name where it was listed and cannot be one here (it holds `\` under the Windows rule, or this host's parser reads it as a drive) names no entry of this host.
- **Lines that name nothing travel apart.** A line that is no relative path of names — `./x`, `../x`, `/abs` — never named a file; every route reported it as a file that did not land and ended unsuccessfully (measured on the build before, below). Such a line cannot go among the names: as text, another host's parser may read it as one (a Windows `.\x` is a file name on Linux), and as a component-form entry it is exactly what the receiver must refuse. It goes in a field of its own, `FilterSpec.files_from_unnamed`, held by the filter as `FileFilter.files_from_unnamed` — in the same form here and at a peer (`split_listed`, used by `build` and `build_spec` alike, for a command's list and a retry's). It is matched against nothing; the source reports each as it does a listed name its scan did not find (`requested_but_unscanned`). A name that cannot be one on the receiving host joins them there.
- **Versioned.** `CONTRACT_VERSION` 12, `SUPPORTED_SPEC_VERSION` 7. The session contract covers push and pull; the request version covers the daemon a daemon-to-daemon transfer is asked of.

Shown against real builds of both contracts (release, the contract-11 build at `cce1f877`), a mirror with a one-name list, all three routes:

```text
contract-11 daemons, contract-12 client
  push / pull        exit 1  session BUILD_MISMATCH: contract version mismatch (D-2026-08-18-2): local v12 vs peer v11
  daemon to daemon   exit 1  delegation rejected by destination daemon: unsupported TransferOperationSpec spec_version 7 (expected 6)
contract-12 daemons, contract-11 client
  push / pull        exit 1  session BUILD_MISMATCH: contract version mismatch (D-2026-08-18-2): local v11 vs peer v12
  daemon to daemon   exit 1  delegation rejected by destination daemon: unsupported TransferOperationSpec spec_version 6 (expected 7)
contract-12 daemons, contract-12 client
  push / pull / daemon to daemon   exit 0, the listed file at the destination
```

And a `copy --retries 0` whose list holds `plain.txt` and one more line, local, push and pull, on the build before and on this one — the same on both, on every route:

```text
./dot.txt, /abs.txt, sub/../up.txt   exit 2, plain.txt copied, "1 file(s) did not land at the destination"
sub//a.txt                           exit 0, plain.txt and sub/a.txt copied
```

A first version of this fix left the lines that name nothing out of what is sent, reasoning that they name nothing anywhere. That probe is what showed it wrong: pushed and pulled, the same list then ran clean and reported success, while locally it still failed.

## Guard proof
- `transfers::filter::tests::a_listed_name_travels_by_its_components`: a list of seven lines — four names written with a doubled separator, an inner `.`, a trailing separator and plainly; three lines that name nothing — is sent as four names in component form and three lines apart; the filter here and the filter a peer builds from the spec answer alike for eight entries and hold the same two parts; a retry's list is told apart the same way. On Windows it also expects `sub\name.txt` sent as `sub/name.txt` (that half runs only there).
- `operation_spec::tests::a_listed_name_off_the_wire_is_read_by_its_components`: `sub/name.txt` is a file in a folder under either platform's rule; `sub\name.txt` is one name under the Unix rule and NO entry under the Windows rule, on every OS; nine malformed entries refuse the list under both rules, naming the entry; through `filter_from_spec`, the listed nested file passes and (off Windows) the file literally called `sub\name.txt` does not — and does when the wire holds the text a Windows sender used to write, which is the reviewer's case; lines that arrive apart stay apart and match nothing.
- `a_request_with_names_listed_by_component_is_version_7_or_later`, `a_listed_name_travels_by_its_components_only_under_contract_12_or_later`: the version-6 request is refused, and the two numbers are held.
- `transfer_session::tests::a_list_line_that_names_nothing_is_reported_as_not_found`: the source's report holds the unnamed lines beside the names its scan did not enumerate, by their text; none without a list.
- Through the binary and a real daemon (`tests/ignore_case.rs`): `a_listed_name_is_the_same_file_at_a_daemon` — local, push, pull: the listed files are copied, an unlisted file is not, and a destination file literally called `sub\name.txt` (off Windows) is kept; `a_list_line_that_names_nothing_is_reported_on_every_route` — three such lines × three routes: the run ends unsuccessfully, names the line as written, and copies the name beside it.
- Mutations, each restored byte for byte: a name sent as this host writes it → the sender guard and the first end-to-end guard red; a line that names nothing sent among the names → the sender guard and the second end-to-end guard; the receiver reading the text as a path of its own host → the receiver guard; a piece holding `\` taken for a name under the Windows rule → the receiver guard; an empty component let through → the receiver guard; `..` let through → the receiver guard; the lines that name nothing dropped on receipt → sender, receiver and second end-to-end guards; the source not reporting them → its unit guard and the second end-to-end guard; request version 6 again → its guard; contract 11 again → its guard.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FilterSpec.files_from_unnamed = 11`; what a `files_from` entry is.
- `crates/blit-core/src/transfers/filter.rs` — `split_listed`; `build` and `build_spec`.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — `listed_name`, `one_name_here`, `filter_from_spec`; spec version 7.
- `crates/blit-core/src/fs_enum.rs` — `FileFilter.files_from_unnamed`.
- `crates/blit-core/src/remote/transfer/source.rs`, `transfer_session/local.rs` — `TransferSource::files_from_unnamed` (provided; wrappers delegate).
- `crates/blit-core/src/transfer_session/mod.rs` — `requested_but_unscanned`; contract 12.
- `crates/blit-daemon/src/service/delegated_pull.rs` — a test's spec literal.
- `crates/blit-cli/tests/ignore_case.rs` — the two end-to-end guards.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`, `docs/TRANSFER_SESSION.md` — contract 12; the fix; the API additions (and two `FilterInputs` fields earlier slices added without listing them).

## Known gaps
- **Nothing here ran on Windows.** The sender's half of the reviewer's case — `sub\name.txt` split into two components — is `std::path` on a Windows host; its expectation is in the guard and runs only there. The receiver's check that this host's own parser reads a piece as one name (a drive-like `C:`) likewise. What is guarded on every OS: the Windows rule that a piece holding `\` is no name, and everything a Linux peer does.
- A Linux name that holds `\`, listed for a Windows peer, names nothing there. A FILE so named, sent to a Windows destination, still lands under the path Windows reads its name as (how manifest paths have always been joined there) — a path the list does not name, so a listed mirror never deletes it.
- A retry's list is built from failure names read as this host's paths (`FilterInputs.retry_only`): a Windows client retrying a name that holds `\` from a Linux source asks for the path Windows reads. Older than this finding and the same root; not changed here.
- What a line that is no relative path of names SHOULD mean is not decided here. rsync 3.5.2, measured: `./dot.txt`, `/abs.txt` and `sub/../up.txt` each copy the file (`dot.txt`, `abs.txt`, `up.txt`). blit reports each as not found — and `find . -type f` writes every line as `./…`. Raised with the owner (`docs/STATE.md`).
- The source's report for a listed name it did not find still reads `missing at retry (not found by the retry scan)` on a first pass.
