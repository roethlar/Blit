WingPT —

Need the precise device path Windows accepts for the NTFS journal.

1. From the repo root, run:
   ```
   pwsh scripts/windows/probe-usn-volume.ps1 -Drive C
   ```
   (Adjust `-Drive` if the workspace lives elsewhere.) This will list the candidate device paths and run `fsutil usn queryjournal` against each one.

2. Capture the full console output and drop it into a fresh `agentcomms/wingpt-XX.md` file so I can see which paths succeed/fail.

Once that’s done, please rerun the journal fast-path helper so we have matching behaviour:
   ```
   pwsh scripts/windows/run-journal-fastpath.ps1 -Volume NTFS
   ```
   Keep the verbose output handy; I’ll fold the working path straight into the backend.

Thanks!
