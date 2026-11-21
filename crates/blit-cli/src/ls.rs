//! `blit ls` — `ls`, run on a daemon's own disk, or on this machine's
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 5, AC-6).
//!
//! Names one per line, sorted as `ls` sorts them under `C.UTF-8` (by
//! their bytes: `A`, `a`, `b`), those beginning with `.` hidden unless
//! `-a`. `-l` prints, for each, the letter of what it is, its size, when
//! it was last written and its name — a link with where it points — after
//! a `total` line, as `ls -l` does, without the columns a listing does not
//! carry (permissions, link count, owner, group). An operand that is no
//! directory is one line: the operand as typed.
//!
//! What cannot be listed is reported in `ls`'s words. The exit status is
//! `ls`'s: 2 when the operand itself could not be listed, 1 when an entry
//! could not be read, 0 otherwise.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::ListArgs;
use blit_core::admin::ls::{self, Entry, EntryPath, ListOptions, Listing};
use blit_core::admin::operand::{word_bytes, Operand};
use blit_core::admin::quote;
use blit_core::disk_usage::{blocks_1k, human_size};
use blit_core::endpoints::{parse_endpoint_or_local, Endpoint};
use blit_core::remote::endpoint::RemotePath;
use eyre::Result;

pub async fn run_ls(args: ListArgs) -> Result<ExitCode> {
    // A bare host names no path: its modules are listed, as `blit list
    // HOST` has always done.
    if let Some(Endpoint::Remote(remote)) = args.target.to_str().map(parse_endpoint_or_local) {
        if matches!(remote.path, RemotePath::Discovery) {
            crate::list_modules::list_modules_remote(remote, args.json).await?;
            return Ok(ExitCode::SUCCESS);
        }
    }
    let options = ListOptions {
        all: args.all,
        // `--json` gives each entry's facts whether or not `-l` was typed.
        facts: args.long || args.json,
        link_itself: args.long,
    };
    // The operand as typed: `ls link/` is the directory the link points
    // to, said by the separator; and a path need not be text.
    let (listing, typed) = match Operand::read(&args.target)? {
        Some(operand) => (
            ls::list(
                &operand.remote,
                operand.module.clone(),
                operand.wire_path(),
                options,
            )
            .await?,
            operand.typed().to_vec(),
        ),
        None => {
            let path = PathBuf::from(&args.target);
            let listing = tokio::task::spawn_blocking(move || ls::list_here(&path, options))
                .await
                .map_err(|join| eyre::eyre!("listing task failed: {join}"))?;
            (listing, word_bytes(&args.target)?.to_vec())
        }
    };

    for failure in &listing.failures {
        eprintln!(
            "blit ls: {} {}: {}",
            failure.action,
            quote::for_shell(&below(&typed, &failure.name), true),
            failure.reason
        );
    }
    let status = if listing.of_the_operand().is_some() {
        ExitCode::from(2)
    } else if listing.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };

    // One write for many lines: the standard output writes each line by
    // itself, which for 200,000 names is 200,000 calls.
    let mut out = std::io::BufWriter::with_capacity(64 * 1024, std::io::stdout().lock());
    let written = if args.json {
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&as_json(&listing, &typed))?
        )
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() as i64);
        let lines = lines(
            &listing,
            &typed,
            args.long,
            args.human_readable,
            &chrono::Local,
            now,
        );
        lines
            .iter()
            .try_for_each(|line| out.write_all(line).and_then(|()| out.write_all(b"\n")))
    };
    match written.and_then(|()| out.flush()) {
        Ok(()) => {}
        // The reader went away (`blit ls … | head -1`): stop, quietly.
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(error) => return Err(error.into()),
    }
    Ok(status)
}

/// An entry of the operand as a message names it: the operand as typed,
/// a separator unless it ends in one, and the entry's name. The operand
/// itself is the operand as typed.
fn below(typed: &[u8], name: &EntryPath) -> Vec<u8> {
    let mut shown = typed.to_vec();
    if name.is_start() {
        return shown;
    }
    if !shown.ends_with(b"/") {
        shown.push(b'/');
    }
    shown.extend_from_slice(name.bytes());
    shown
}

/// One line to print: an entry's name — the operand as typed, for the
/// operand itself — and its facts.
struct Row<'a> {
    name: &'a [u8],
    entry: &'a Entry,
}

/// The rows of a listing, in `ls`'s order: by the names' bytes, which is
/// the order of their characters under `C.UTF-8` (`A`, `a`, `b`).
fn rows<'a>(listing: &'a Listing, typed: &'a [u8]) -> Vec<Row<'a>> {
    if let Some(itself) = &listing.itself {
        return vec![Row {
            name: typed,
            entry: itself,
        }];
    }
    let mut rows: Vec<Row<'a>> = listing
        .entries
        .iter()
        .map(|entry| Row {
            name: entry.name.bytes(),
            entry,
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(b.name));
    rows
}

/// What `blit ls` prints, line by line, each as its bytes.
fn lines<Tz>(
    listing: &Listing,
    typed: &[u8],
    long: bool,
    human: bool,
    zone: &Tz,
    now_seconds: i64,
) -> Vec<Vec<u8>>
where
    Tz: chrono::TimeZone,
    Tz::Offset: std::fmt::Display,
{
    // An operand that could not be listed prints nothing.
    if listing.of_the_operand().is_some() {
        return Vec::new();
    }
    let rows = rows(listing, typed);
    if !long {
        return rows.iter().map(|row| row.name.to_vec()).collect();
    }
    let size = |bytes: u64| {
        if human {
            human_size(bytes)
        } else {
            bytes.to_string()
        }
    };
    let sizes: Vec<String> = rows
        .iter()
        .map(|row| match &row.entry.facts {
            Some(facts) => size(facts.size),
            None => "?".to_string(),
        })
        .collect();
    let width = sizes.iter().map(String::len).max().unwrap_or(0);
    let mut lines = Vec::with_capacity(rows.len() + 1);
    // A directory's listing begins with the space its entries take, in
    // 1K blocks, as `ls -l` begins.
    if listing.itself.is_none() {
        let allocated: u64 = rows
            .iter()
            .filter_map(|row| row.entry.facts.as_ref())
            .map(|facts| facts.allocated_bytes)
            .sum();
        let total = if human {
            human_size(allocated)
        } else {
            blocks_1k(allocated).to_string()
        };
        lines.push(format!("total {total}").into_bytes());
    }
    for (row, size) in rows.iter().zip(&sizes) {
        let (letter, time) = match &row.entry.facts {
            Some(facts) => (
                facts.kind.letter(),
                blit_core::listing::time_column(zone, facts.mtime_seconds, now_seconds),
            ),
            // Its facts could not be read, which was reported: `ls`
            // prints the name all the same, with `?` for what is missing.
            None => ('?', "?".to_string()),
        };
        let mut line = format!("{letter} {size:>width$} {time:>12} ").into_bytes();
        line.extend_from_slice(row.name);
        if let Some(target) = row
            .entry
            .facts
            .as_ref()
            .and_then(|facts| facts.link_target.as_ref())
        {
            line.extend_from_slice(b" -> ");
            line.extend_from_slice(target.bytes());
        }
        lines.push(line);
    }
    lines
}

/// `--json`: what the operand is, each entry with its facts, and what
/// could not be listed. A name that is not text has its bytes beside it
/// (`raw`, as `blit_core::admin::quote::exact` writes them).
fn as_json(listing: &Listing, typed: &[u8]) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = rows(listing, typed)
        .iter()
        .map(|row| {
            let mut entry = serde_json::json!({
                "name": String::from_utf8_lossy(row.name),
            });
            if let Some(raw) = quote::exact(row.name) {
                entry["raw"] = raw.into();
            }
            if let Some(facts) = &row.entry.facts {
                entry["kind"] = facts.kind.word().into();
                entry["is_dir"] = (facts.kind == ls::Kind::Directory).into();
                entry["size"] = facts.size.into();
                entry["mtime_seconds"] = facts.mtime_seconds.into();
                if let Some(target) = &facts.link_target {
                    entry["target"] = String::from_utf8_lossy(target.bytes()).into();
                    if let Some(raw) = quote::exact(target.bytes()) {
                        entry["raw_target"] = raw.into();
                    }
                }
            }
            entry
        })
        .collect();
    let failures: Vec<serde_json::Value> = listing
        .failures
        .iter()
        .map(|failure| {
            quote::in_json(
                &below(typed, &failure.name),
                serde_json::json!({ "action": failure.action, "reason": failure.reason }),
            )
        })
        .collect();
    serde_json::json!({
        "listed_into": listing.itself.is_none() && listing.of_the_operand().is_none(),
        "entries": entries,
        "failures": failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::admin::ls::{EntryFacts, Failure, Kind};

    fn entry(name: &[u8], kind: Kind, size: u64, when: &str, target: Option<&str>) -> Entry {
        Entry {
            name: EntryPath::typed(name),
            facts: Some(EntryFacts {
                kind,
                size,
                mtime_seconds: at(when),
                allocated_bytes: size.div_ceil(4096) * 4096,
                link_target: target.map(|target| EntryPath::typed(target.as_bytes())),
            }),
        }
    }

    fn at(text: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(text)
            .unwrap()
            .timestamp()
    }

    fn text(lines: Vec<Vec<u8>>) -> Vec<String> {
        lines
            .into_iter()
            .map(|line| String::from_utf8_lossy(&line).into_owned())
            .collect()
    }

    const NOW: &str = "2026-11-21T11:32:00Z";
    const RECENT: &str = "2026-11-21T11:30:00Z";

    /// A directory's entries in the order a directory gives them: none.
    fn directory() -> Listing {
        Listing {
            itself: None,
            entries: vec![
                entry(b"sub", Kind::Directory, 60, "2026-11-20T10:43:00Z", None),
                entry(b"k15", Kind::File, 1500, RECENT, None),
                entry(b"a.txt", Kind::File, 6, "2026-11-20T10:43:00Z", None),
                entry(b"A.txt", Kind::File, 1, "2025-01-02T03:04:05Z", None),
                entry(b"m1", Kind::File, 1_048_576, RECENT, None),
                entry(b"link", Kind::Link, 3, "2026-11-20T10:43:00Z", Some("sub")),
                entry("é.txt".as_bytes(), Kind::File, 1, RECENT, None),
                entry(b"bad\xffname", Kind::File, 1, RECENT, None),
                entry(b"b", Kind::File, 0, "2027-06-01T00:00:00Z", None),
                entry(b"Z", Kind::File, 1, RECENT, None),
                entry(b"pipe", Kind::Fifo, 0, RECENT, None),
                entry(b"_u", Kind::File, 1, RECENT, None),
                entry(b"~t", Kind::File, 1, RECENT, None),
                entry(b"0n", Kind::File, 1, RECENT, None),
            ],
            failures: Vec::new(),
        }
    }

    /// AC-6: names one per line, in the order GNU ls 9.12 printed these
    /// under `C.UTF-8` (run 2026-11-21) — by their bytes, a name that is
    /// not text among them by its own.
    #[test]
    fn names_are_sorted_as_ls_sorts_them() {
        let listing = directory();
        let printed = lines(&listing, b"t", false, false, &chrono::Utc, at(NOW));
        let names: Vec<&[u8]> = printed.iter().map(Vec::as_slice).collect();
        let expected: [&[u8]; 14] = [
            b"0n",
            b"A.txt",
            b"Z",
            b"_u",
            b"a.txt",
            b"b",
            b"bad\xffname",
            b"k15",
            b"link",
            b"m1",
            b"pipe",
            b"sub",
            b"~t",
            "é.txt".as_bytes(),
        ];
        assert_eq!(names, expected);
    }

    /// AC-6: the long form, as GNU ls 9.12 printed these entries under
    /// `C.UTF-8` and `TZ=UTC` at 2026-11-21T11:32Z, less the columns a
    /// listing does not carry: the letter, the size lined up on the right,
    /// the time, the name, and a link's target.
    #[test]
    fn the_long_form_is_ls_s_without_what_a_listing_does_not_carry() {
        let listing = directory();
        let printed = text(lines(&listing, b"t", true, false, &chrono::Utc, at(NOW)));
        assert_eq!(
            printed,
            [
                "total 1068",
                "-       1 Nov 21 11:30 0n",
                "-       1 Jan  2  2025 A.txt",
                "-       1 Nov 21 11:30 Z",
                "-       1 Nov 21 11:30 _u",
                "-       6 Nov 20 10:43 a.txt",
                "-       0 Jun  1  2027 b",
                "-       1 Nov 21 11:30 bad\u{fffd}name",
                "-    1500 Nov 21 11:30 k15",
                "l       3 Nov 20 10:43 link -> sub",
                "- 1048576 Nov 21 11:30 m1",
                "p       0 Nov 21 11:30 pipe",
                "d      60 Nov 20 10:43 sub",
                "-       1 Nov 21 11:30 ~t",
                "-       1 Nov 21 11:30 é.txt",
            ]
        );
        // `-h`: sizes as `ls -lh` printed them, and the total likewise.
        let human = text(lines(&listing, b"t", true, true, &chrono::Utc, at(NOW)));
        assert_eq!(human[0], "total 1.1M");
        assert_eq!(human[8], "- 1.5K Nov 21 11:30 k15");
        assert_eq!(human[10], "- 1.0M Nov 21 11:30 m1");
        assert_eq!(human[5], "-    6 Nov 20 10:43 a.txt");
        // An empty directory: its total, and nothing else; and nothing at
        // all without the long form.
        let empty = Listing::default();
        assert_eq!(
            text(lines(&empty, b"t", true, false, &chrono::Utc, at(NOW))),
            ["total 0"]
        );
        assert!(lines(&empty, b"t", false, false, &chrono::Utc, at(NOW)).is_empty());
    }

    /// AC-6: an operand that is no directory is one line — the operand as
    /// typed — with no `total` before it; and one that could not be
    /// listed prints nothing.
    #[test]
    fn an_operand_that_is_no_directory_is_printed_as_typed() {
        let mut file = entry(b"", Kind::File, 6, "2026-11-20T10:43:00Z", None);
        file.name = EntryPath::default();
        let listing = Listing {
            itself: Some(file),
            ..Listing::default()
        };
        let typed = b"host:/m/t/a.txt";
        assert_eq!(
            text(lines(&listing, typed, false, false, &chrono::Utc, at(NOW))),
            ["host:/m/t/a.txt"]
        );
        assert_eq!(
            text(lines(&listing, typed, true, false, &chrono::Utc, at(NOW))),
            ["- 6 Nov 20 10:43 host:/m/t/a.txt"]
        );
        let mut link = entry(b"", Kind::Link, 3, "2026-11-20T10:43:00Z", Some("sub"));
        link.name = EntryPath::default();
        let listing = Listing {
            itself: Some(link),
            ..Listing::default()
        };
        assert_eq!(
            text(lines(
                &listing,
                b"t/link",
                true,
                false,
                &chrono::Utc,
                at(NOW)
            )),
            ["l 3 Nov 20 10:43 t/link -> sub"]
        );
        let refused = Listing {
            failures: vec![Failure {
                action: "cannot access".to_string(),
                name: EntryPath::default(),
                reason: "No such file or directory".to_string(),
            }],
            ..Listing::default()
        };
        assert!(lines(&refused, b"t/missing", true, false, &chrono::Utc, at(NOW)).is_empty());
        assert!(refused.of_the_operand().is_some());
    }

    /// An entry whose facts could not be read is still printed, with `?`
    /// where they would stand; and a message names it below the operand.
    #[test]
    fn an_entry_without_its_facts_is_still_a_line() {
        let listing = Listing {
            itself: None,
            entries: vec![
                entry(b"seen", Kind::File, 12, RECENT, None),
                Entry {
                    name: EntryPath::typed(b"gone"),
                    facts: None,
                },
            ],
            failures: vec![Failure {
                action: "cannot access".to_string(),
                name: EntryPath::typed(b"gone"),
                reason: "No such file or directory".to_string(),
            }],
        };
        assert_eq!(
            text(lines(&listing, b"t", true, false, &chrono::Utc, at(NOW))),
            [
                "total 4",
                "?  ?            ? gone",
                "- 12 Nov 21 11:30 seen",
            ]
        );
        assert!(listing.of_the_operand().is_none());
        assert_eq!(below(b"h:/m/t", &listing.failures[0].name), b"h:/m/t/gone");
        assert_eq!(below(b"h:/m/t/", &listing.failures[0].name), b"h:/m/t/gone");
        assert_eq!(below(b"h:/m/t", &EntryPath::default()), b"h:/m/t");
    }

    /// `--json`: each entry with its facts, a name that is not text with
    /// its bytes beside it, and what could not be listed.
    #[test]
    fn json_gives_each_entry_its_facts_and_a_name_that_is_not_text_its_bytes() {
        let listing = directory();
        let json = as_json(&listing, b"t");
        assert_eq!(json["listed_into"], true);
        let entries = json["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 14);
        assert_eq!(entries[0]["name"], "0n");
        assert_eq!(entries[0]["kind"], "file");
        assert_eq!(entries[0]["is_dir"], false);
        let bad = &entries[6];
        assert_eq!(bad["name"], "bad\u{fffd}name");
        assert_eq!(bad["raw"], "bad\\xffname");
        let link = &entries[8];
        assert_eq!(
            (link["kind"].as_str(), link["target"].as_str()),
            (Some("link"), Some("sub"))
        );
        assert_eq!(entries[11]["is_dir"], true);
        assert_eq!(entries[7]["size"], 1500);
        assert!(entries[0].get("raw").is_none() && entries[0].get("target").is_none());
        assert!(json["failures"].as_array().unwrap().is_empty());
    }
}
