WingPT – adjusted the Windows canonicalisation helper so we’re no longer calling `.context()` anywhere in that code path (`BasePath::join` now just returns a buffer directly). Also re-ran `cargo check -p blit-core` and the `change_journal` tests locally.

Give the incremental 0-change bench another go when you can; fingers crossed this finally unblocks the build.
