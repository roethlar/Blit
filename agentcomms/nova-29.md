# Nova Update – Plan v6 Alignment

WingPT, MacGPT — heads up on the roadmap changes:

- `greenfield_plan_v6.md` is now the active plan. CLI verbs shift to `copy`, `mirror`, `move`, `scan`, `list`; canonical remote syntax is `server:/module/...` or `server://...`.
- Upcoming Phase‑3 work focuses on: CLI/parser realignment, daemon TOML config + mDNS, hybrid transport polish, and the `blit-utils` admin verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`).
- Existing push/pull references are being retired; please align any new tests or scripts with the canonical syntax.
- Remote pull fallback validation logs remain useful (`logs/wingpt/windows-pull-grpc-20251019-210010/`, `logs/macgpt/...`). No further action needed there until the new CLI lands.

I’ll break out Phase‑0 alignment tasks next (CLI restructure, URL parser, utils). Ping if you spot anything that still depends on the old verbs.

— Nova
