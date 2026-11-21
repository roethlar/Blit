# cr-ecm10-1: The line concluded from one name what holds at every path

**Severity**: HIGH — the question was asked with no line, or with a line about another pattern, and a confirmed mirror deleted what the other mode keeps
**Status**: Reviewed, found incomplete — `exclude-case-r11` (Codex openreview, 2026-11-21) reviewed `d3e33acc..bfa537f9`, the six `exclude-case-r10` fixes, with `guard_confirmed: true`. One path still shows an example it did not check: when the pattern as the option would read it cannot be compiled at all (an `s` inside 249 nested brace pairs), the line offers an upper-case spelling, and the same command with `--ignore-case` is refused at validation. The remaining work is cr-ecm11-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c77dd27d`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. The second of four commits that complete cr-ecm9-2.

## Evidence
With `--exclude a --exclude ./A`, the head printed no case warning. The default mirror deleted `sub/A`, while `--ignore-case` kept it. The reverse risk also occurs. With excludes `Keep`, `[!a]*.txt`, and `Ax.txt`, the prompt advertised additional spellings of `Keep`, yet `--ignore-case` deleted `Ay.txt`, which the default kept. The reviewer: a root-level example or one expansion of `*` cannot establish what happens at every path. When coverage is uncertain, retain a warning that protection may shrink.

Cause. Three places took "no difference at the names tried" for "no difference":
- **A pattern was called unchanged when the LIST named every spelling of it** (cr-ecm9-2's `compared`, asked of the whole list as typed). The names were tried at the top of the tree, where `./A` does keep `A`; one level down it does not. Called unchanged, `a` was neither shown nor counted, and with nothing else to show there was no line.
- **A class that could take less was passed over when the name tried was still kept** — by another pattern, at that one path, for that one text in place of `*` (`Ax.txt`).
- **And then the line spoke of something else**, a gain, as if no loss were possible.

## What eight rounds show
Each round since the third made the line check more of what it says against the filter; each check was sound for the cases it was written for and unsound beside them. What a set of globs matches over every path is not settled by trying names. So the line no longer concludes anything. It shows what a concrete name bears out, and what it cannot show it says in words that are true of every pattern with a letter in it.

## Intake
Admitted as the reviewer put it. The design above is the agent's; the simpler alternative — the plain rule alone, always — is put to the owner in `docs/STATE.md`.

## What
- **A pattern is left out only when its OWN two readings are proved the same** (`Asked::compared`, asked of the pattern alone): `{a,A}`, `[Cc]`. Never because another pattern seems to cover it. `--exclude a --exclude A` and `--exclude a --exclude ./A` both get `"a" matches exactly as typed, and 1 more pattern matches only as typed; capital letters matter (--ignore-case ignores them).` — no spelling is offered, none being borne out, and the line stays.
- **Where a class could take less, the warning stays.** A letter is said to be lost — `would stop matching` — when a name shows the run stops naming it; when none does, the class the option narrows by its text is still named, and the line says `may stop matching`. That pattern is shown before any gain.
- **A `*` is tried as some text and as none** (`STARS`). With `Ax.txt` kept by another pattern, `A.txt` shows what `[!a]*.txt` gives up: `[!a] would stop matching A`.
- An example from a pattern of alternatives (`ς` for `{Σ,σ}`) is one the run would treat otherwise, like any other.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the lines to the letter — `a` beside `A`, beside `./A`, and with `{b,B}` (still not counted); `{b,B}` with `[Cc]` (no line: nothing reads otherwise); `[!a].dat` beside `A.dat`, `{[!a].1,A.1}` and `x{[!a],A}` (`may`); `Keep`, `[!a]*.txt`, `Ax.txt` (`would stop matching A`).
- `case_fold::tests::each_claim_is_checked_against_the_filter`: `compared` beside `A` and beside `./A` — the readings differ, and no example is borne out; the loss at `[!a]*.txt` beside `Ax.txt`; names read with `*` as none.
- `case_fold::tests::the_case_note_says_what_the_first_lettered_pattern_matches`: a pattern beside `*` keeps its line, in words.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary — the reviewer's two commands: the line, then both mirrors. `a` and `./A`: `sub/A` deleted as typed, kept with the option. `Keep`, `[!a]*.txt`, `Ax.txt`: `Ay.txt` kept as typed, deleted with the option.
- Mutations, each restored byte for byte: a pattern called the same when the list as typed names every spelling, as before → all three of the first, second and end-to-end guards red; a class that could take less not named when no name shows it → the first; a `*` tried as one text only → the first, second and end-to-end.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `case_note_of` (the pattern that may narrow; `would`/`may`), `Spellings::with` and `narrowing_borne_out`, `Asked::compared`, `Asked::lost_here`, `STARS`, `samples_with`; the guards.
- `crates/blit-cli/tests/case_note.rs` — the two cases; one line's count.
- `CHANGELOG.md`.

## Known gaps
- **`may` can be said where nothing is lost**: `{[!a].1,A.1}` keeps `A.1` through the alternative beside the class, and the line still says `[!a] may stop matching A`. Telling that apart needs every name the pattern can match, which a wildcard or a second class puts out of reach.
- **A line can be shown where the option changes nothing**: `--exclude a --exclude A`. It claims nothing about any name.
- A name is tried at the top of the tree, with `*` as `x` and as nothing. A loss that shows only deeper, or for another text, is said with `may` when the pattern holds a class that narrows, and not at all otherwise — a pattern can only match LESS with the option through such a class.
- The checks run under this machine's path rule. Run on Linux only.
