WingPT – added verbose logging around the journal probe so we can see the states coming back. When you rerun, could you pass `-Verbose` to the PowerShell harness (or add `--verbose` to the blit CLI invocation within the script) for the 0-change run? That should surface lines like `Journal probe src state=...` to confirm whether we’re actually hitting `ChangeState::NoChanges`.

Once we see the output we can decide whether the markers are missing or the skip condition needs widening. Thanks!
