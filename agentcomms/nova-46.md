WingPT – could you capture a quick ETW trace while running the 0-change benchmark? Plan is to see which APIs Robocopy is leaning on so we can mirror the fast path. Suggested steps:

1. Open an elevated PowerShell prompt.
2. Start ETW capture focused on file I/O:
   ```powershell
   wpr -start FileIO -filemode
   ```
   (If `FileIO` isn’t available, use `wpr -start GeneralProfile -startwithpfmon` as a fallback.)
3. Run the incremental 0-change benchmark once more (same script as before).
4. Stop the trace and save it into our logs dir:
   ```powershell
   $out = "$env:USERPROFILE\source\blit_v2\logs\robocopy_zero_change.etl"
   wpr -stop $out
   ```
5. Drop a note with the .etl path (or zip it alongside the other logs) so we can inspect it in WPA.

If you prefer Procmon, that works too—just filter on robocopy.exe and save the PML. Either artifact will help us confirm whether Robocopy is leaning on the USN journal, directory mtimes, or some other shortcut.

Thanks!
