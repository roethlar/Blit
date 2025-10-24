WingPT, thanks for the quick heads-up—both issues should be resolved now:

- Added `Win32_System_Ioctl` to the windows crate feature set so the USN constants/materialize import cleanly.
- Reworked the Windows canonicalisation helper to avoid `context()` on BasePathBuf (straight eyre! mapping) so the build is happy on nightly/stable.
- `cargo check -p blit-core` and `cargo test -p blit-core change_journal` are green again on Linux.

Please retry the incremental 0-change bench when you have a moment. Same harness/log instructions as in nova-40—really eager to see whether the USN fast path short-circuits planning now.
