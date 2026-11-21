//! A utility verb's operand below its module, read as it was typed
//! (cr-uv1-1).
//!
//! `rm`, `du`, `df` and `find` are the Unix commands run on a daemon's own
//! disk, and to those commands HOW a path is written says something: `x/`
//! names `x` only if it is a directory, and through a link; `x/.` likewise;
//! `rm` refuses a path whose last name is `.` or `..`; a space at either
//! end is part of a name. A path that was tidied on the way — its last
//! separator dropped, a `.` folded away, its ends trimmed — is another
//! path: `rm -rf HOST:/m/work/.` deleted `work`, `rm HOST:/m/doc/` unlinked
//! a file, and of `report` and `report ` (a space after it) the wrong one
//! went.
//!
//! So the path goes from the command line to the daemon as typed, and what
//! its writing says is read here, by both ends, from that one text.
//! Separators are `/`: a remote path is written with forward slashes on
//! every machine.

/// The last name of a typed path: the text after its last separator, the
/// separators after that name set aside. Empty for a path that holds no
/// name at all.
pub fn final_name(typed: &str) -> &str {
    typed
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
}

/// The path's last name is `.` or `..` — what `rm` refuses to remove,
/// whatever its options (`refusing to remove '.' or '..' directory`).
pub fn names_dot(typed: &str) -> bool {
    matches!(final_name(typed), "." | "..")
}

/// The path is written so that it names a directory and nothing else: with
/// a separator after it, or with `.` as its last name. The system then
/// resolves a link standing there and refuses anything that is no
/// directory (`Not a directory`).
pub fn names_a_directory(typed: &str) -> bool {
    typed.ends_with('/') || final_name(typed) == "."
}

/// The path holds no name: only separators and `.`. It is the module
/// itself.
pub fn is_the_root(typed: &str) -> bool {
    typed.split('/').all(|part| part.is_empty() || part == ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cr-uv1-1: what the writing of a path says, read from the text as
    /// typed — each row a path, its last name, and whether it ends in `.`
    /// or `..`, is written as a directory, is the module itself.
    #[test]
    fn a_typed_path_says_how_it_was_written() {
        for (typed, last, dot, directory, root) in [
            ("work", "work", false, false, false),
            ("work/", "work", false, true, false),
            ("work//", "work", false, true, false),
            ("work/.", ".", true, true, false),
            ("work/./", ".", true, true, false),
            ("work//.", ".", true, true, false),
            ("work/..", "..", true, false, false),
            ("work/sub/../", "..", true, true, false),
            ("a/b/c", "c", false, false, false),
            ("a/./b", "b", false, false, false),
            // A space is part of a name, at either end.
            ("report ", "report ", false, false, false),
            (" report", " report", false, false, false),
            ("dir/ ", " ", false, false, false),
            // A name that only looks like `.`.
            ("...", "...", false, false, false),
            (".hidden", ".hidden", false, false, false),
            ("x/. ", ". ", false, false, false),
            // No name at all: the module.
            ("", "", false, false, true),
            (".", ".", true, true, true),
            ("./", ".", true, true, true),
            ("/", "", false, true, true),
            ("././/.", ".", true, true, true),
        ] {
            assert_eq!(final_name(typed), last, "{typed:?}");
            assert_eq!(names_dot(typed), dot, "{typed:?}");
            assert_eq!(names_a_directory(typed), directory, "{typed:?}");
            assert_eq!(is_the_root(typed), root, "{typed:?}");
        }
    }
}
