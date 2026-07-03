---
description: Run a named durable playbook from .agents/playbooks/
argument-hint: [name]
---
Run the `playbook` operator defined in `AGENTS.md` (Operator Requests): read
the named playbook at `.agents/playbooks/$ARGUMENTS.md` and follow it exactly.
If the named playbook does not exist, say so rather than guessing.
