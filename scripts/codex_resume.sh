#!/usr/bin/env bash
set -euo pipefail
source ~/source/venvs/superclaude/bin/activate  
sudo npm install -g @openai/codex
pipx upgrade SuperClaude
codex resume 019a5ba3-6704-7bc3-996b-0e76a3944199 -s danger-full-access -a never
