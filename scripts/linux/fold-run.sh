#!/usr/bin/env bash
# Run a command with its temporary files on a filesystem that folds case — as
# macOS (APFS) and Windows (NTFS) do — on Linux, without root.
#
#   scripts/linux/fold-run.sh cargo test --workspace --no-fail-fast
#
# How: a tmpfs mounted `casefold` in a user and mount namespace of the
# command's own, and the command run as the calling user again in a nested
# namespace, so that permission checks are an ordinary user's. TMPDIR points
# into the mount for the command; nothing stays mounted afterwards.
#
# What it is like: it folds case fully (`straße` is `STRASSE`) and pairs `ſ`
# with `s`, as APFS does; NTFS folds less. It is not macOS or Windows — the
# suite run there is still the evidence (.agents/repo-guidance.md,
# Verification) — but it finds a guard that assumes a filesystem keeps case
# apart before CI does.
#
# Needs: a kernel whose tmpfs takes `casefold` (6.13 on) and unprivileged
# user namespaces. Exit 90: the mount does not fold case here.
#
# Known: a test that gives a file another of the caller's groups fails inside
# (the nested namespace maps one group) and passes outside it.
set -euo pipefail
[ $# -gt 0 ] || { echo "usage: $0 <command> [args...]" >&2; exit 2; }
mnt="$(mktemp -d "${TMPDIR:-/tmp}/blit-fold.XXXXXX")"
trap 'rmdir "$mnt" 2>/dev/null || true' EXIT
FOLD_UID="$(id -u)"
FOLD_GID="$(id -g)"
export FOLD_UID FOLD_GID
unshare -rm bash -c '
  set -euo pipefail
  mount -t tmpfs -o casefold tmpfs "$0"
  mkdir "$0/tmp"
  chattr +F "$0/tmp"
  touch "$0/tmp/Probe"
  [ -e "$0/tmp/PROBE" ] || { echo "fold-run: the mount does not fold case" >&2; exit 90; }
  rm "$0/tmp/Probe"
  export TMPDIR="$0/tmp"
  exec unshare -U --map-user="$FOLD_UID" --map-group="$FOLD_GID" "$@"
' "$mnt" "$@"
