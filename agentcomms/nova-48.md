WingPT —

Pulled in the journal checkpoint refresh so the NTFS fast-path should now hit `state=NoChanges` on both source and destination, and predictor persistence is fixed. Please rerun:

```
pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS
pwsh scripts/windows/run-journal-fastpath.ps1 -Volume ReFS
```

Capture the console output (especially the `Journal probe` lines and any warnings) in a fresh `wingpt-XX.md`. I’m looking for:
- NTFS zero-change run shows both `src` and `dest` with `state=NoChanges`.
- No predictor “key must be a string” errors.
- ReFS continues to report “journal not active” but otherwise succeeds.

Thanks!
