#!/usr/bin/env bash
set -euo pipefail

# === Config ===
CLAUDE_SRC_DEFAULT="${HOME}/.claude/commands/sc"
CODEX_DIR="${HOME}/.codex"
PROMPTS_DIR="${CODEX_DIR}/prompts"
SG_DIR="${CODEX_DIR}/commands/sg"
STAGING="${HOME}/codex_ported_all"          # bundle build dir
ZIP_OUT="${HOME}/codex_ported_all.zip"      # resulting zip

# Allow override: ./make_codex_bundle.sh /path/to/claude/sc install|zip|both
CLAUDE_SRC="${1:-$CLAUDE_SRC_DEFAULT}"
MODE="${2:-both}"          # install | zip | both

say(){ printf "%b\n" "$*"; }
title(){ printf "\n\033[1m%s\033[0m\n" "$*"; }

need_python(){
  command -v python3 >/dev/null || { echo "❌ python3 not found"; exit 1; }
}

ensure_dirs(){
  mkdir -p "$STAGING/prompts"
  mkdir -p "$STAGING/bin"
}

# ——— Python transformer ———
# Converts a SuperClaude .md to Codex:
#  - first header => "# sc.<name>"
#  - replace "/sc:" references with "/prompts:sc."
#  - add "## Invocation" if missing
#  - add "## Procedure (execution plan)" if missing (the "doer" block)
py_porter(){
python3 - "$@" <<'PY'
import sys, re, os, io, pathlib

def port_text(txt, name):
    # normalize header
    header = f"# sc.{name}"
    lines = txt.lstrip().splitlines()
    if not lines or not lines[0].startswith("# "):
        lines = [header, ""] + lines
    else:
        lines[0] = header

    out = "\n".join(lines)

    # replace invocation tokens
    out = re.sub(r"/sc:", "/prompts:sc.", out)

    # ensure Invocation
    if "## Invocation" not in out:
        out += f"\n\n## Invocation\n`/prompts:sc.{name} [args]`\n"

    # ensure Procedure
    if not re.search(r"^## +Procedure\\b", out, flags=re.IGNORECASE|re.MULTILINE):
        out += """
---
## Procedure (execution plan)

You are **Codex** operating interactively in this repository.
Perform the workflow described above — actually read/grep/analyze files, generate or edit code,
run safe build/test/lint commands, and summarize results.
Do **not** just describe the process; execute it now.
If no specific target was given, act on the current directory.

Begin immediately.
"""
    return out

def port_file(src_path, dest_path):
    name = os.path.splitext(os.path.basename(src_path))[0].strip().replace(" ","-")
    with open(src_path, "r", encoding="utf-8", errors="replace") as f:
        txt = f.read()
    fixed = port_text(txt, name)
    with open(dest_path, "w", encoding="utf-8") as f:
        f.write(fixed)

for i in range(1, len(sys.argv), 2):
    src = sys.argv[i]
    dst = sys.argv[i+1]
    port_file(src, dst)
PY
}

make_sc_prompts(){
  title "🧩 Porting SuperClaude prompts → Codex"
  shopt -s nullglob
  files=("$CLAUDE_SRC"/*.md)
  if (( ${#files[@]} == 0 )); then
    say "⚠️  No SuperClaude prompts found in: $CLAUDE_SRC"
  else
    args=()
    for f in "${files[@]}"; do
      base="$(basename "$f" .md)"
      dest="${STAGING}/prompts/sc.${base}.md"
      args+=("$f" "$dest")
    done
    py_porter "${args[@]}"
    say "• Ported ${#files[@]} SuperClaude prompts to ${STAGING}/prompts"
  fi
  shopt -u nullglob
}

make_sg_wrappers(){
  title "🧩 Creating sg.* wrappers (from ~/.codex/commands/sg/*.toml)"
  if [[ ! -d "$SG_DIR" ]]; then
    say "⚠️  ${SG_DIR} not found. Wrappers will still be created using standard task names."
  fi

  # Task list from your environment
  tasks=(analyze build cleanup design document estimate explain git implement improve index load reflect save select-tool test troubleshoot)
  for t in "${tasks[@]}"; do
    cat > "${STAGING}/prompts/sg.${t}.md" <<EOF
# sg.${t} – SG Task Wrapper

Execute the intent of **sg \`${t}\`** using Codex tools.
If the external \`sg\` binary is missing, emulate using the definition at \`~/.codex/commands/sg/${t}.toml\`.
Read the TOML to infer steps, then act.

## Invocation
\`/prompts:sg.${t} [args]\`

## Steps
1) Inspect \`~/.codex/commands/sg/${t}.toml\` to infer goals/inputs.
2) Use Glob/Grep/Read to locate relevant code/config.
3) Apply changes with Write/MultiEdit when required and show small diffs.
4) Summarize outputs and propose next steps.

---
## Procedure (execution plan)
You are **Codex** operating interactively in this repository.
Perform the workflow above — actually read/grep/analyze files, generate or edit code,
run safe build/test/lint commands, and summarize results.
Do **not** just describe; execute now. If no target given, act on current directory.
EOF
  done
  say "• Created ${#tasks[@]} sg.* wrappers in ${STAGING}/prompts"
}

add_util_prompts(){
  title "➕ Adding continue & hello prompts"
  cat > "${STAGING}/prompts/continue.md" <<'EOF'
# continue – Execute "Next Steps" from the previous message

Use the previous Codex reply as context. Parse its "Next Steps"/numbered items and execute them in order.

## Invocation
`/prompts:continue [--simulate|--safe|--full]`

## Procedure
1) Recover the previous message; extract actionable steps (shell/code/doc).
2) Execute (or simulate); capture short logs.
3) Validate; summarize success/failure and remaining steps.
Begin now.
EOF

  cat > "${STAGING}/prompts/hello.md" <<'EOF'
# hello
Say exactly: "Hello from a Codex custom prompt." Then stop.
EOF
}

write_verify(){
  cat > "${STAGING}/verify_install.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "Prompts in this bundle:"
ls -1 prompts | sort | wc -l
ls -1 prompts | sort | head -n 80
echo
echo "To test in Codex:"
echo "  codex"
echo "  /prompts:hello"
echo "  /prompts:continue --simulate"
echo "  /prompts:sc.analyze ."
echo "  /prompts:sg.analyze"
EOF
  chmod +x "${STAGING}/verify_install.sh"
}

zip_bundle(){
  title "📦 Building ZIP → ${ZIP_OUT}"
  rm -f "$ZIP_OUT"
  (cd "$STAGING" && zip -qr "$ZIP_OUT" .)
  say "✅ ZIP ready: ${ZIP_OUT}"
}

install_inplace(){
  title "🛠  Installing in-place to ${PROMPTS_DIR}"
  mkdir -p "${PROMPTS_DIR}"
  BK="${PROMPTS_DIR}/_backup_$(date +%Y%m%d_%H%M%S)"
  cp -a "${PROMPTS_DIR}/." "${BK}/" 2>/dev/null || true
  say "• Backup saved: ${BK}"
  cp -a "${STAGING}/prompts/." "${PROMPTS_DIR}/"
  say "• Installed ${STAGING}/prompts → ${PROMPTS_DIR}"
}

main(){
  need_python
  ensure_dirs
  make_sc_prompts
  make_sg_wrappers
  add_util_prompts
  write_verify

  case "$MODE" in
    install) install_inplace ;;
    zip)     zip_bundle ;;
    both)    install_inplace; zip_bundle ;;
    *)       echo "Unknown mode: $MODE (use install|zip|both)"; exit 1 ;;
  esac

  title "✅ Done"
  say "Next steps:"
  [[ "$MODE" != "zip" ]] && say "  codex  # then try /prompts:hello, /prompts:sc.analyze ., /prompts:continue --simulate"
  [[ "$MODE" != "install" ]] && say "  unzip ${ZIP_OUT} -d ~/.codex  # to install elsewhere"
}
main "$@"
