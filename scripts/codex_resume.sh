#!/usr/bin/env bash
set -euo pipefail
source ~/source/venvs/superclaude/bin/activate  
sudo npm install -g @openai/codex
pipx upgrade SuperClaude
codex resume 0199f419-b5b4-7422-839b-3be13b973ea8 -s danger-full-access -a never
