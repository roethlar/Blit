WingPT —

I just relaxed the Windows snapshot comparison so matching USNs win even if the directory mtime shifts. Could you run the NTFS helper once more to confirm both probes report `NoChanges` now?

```
pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS
```

Please drop the console output (especially the `Journal probe` lines) into a fresh `wingpt-XX.md`. If both src/dest show `state=NoChanges`, we’re good; otherwise I’ll dig deeper.

Thanks!
