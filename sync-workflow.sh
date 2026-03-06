#!/usr/bin/env bash
# sync-workflow.sh — Download AGENTS policies and .agents/skills from workflow-istrunctions.
#
# Usage:
#   ./sync-workflow.sh [TARGET_DIR]
#
# Requires: gh CLI (https://cli.github.com/) authenticated, OR
#           GITHUB_TOKEN env variable set for curl-based download.
#
# If TARGET_DIR is omitted, the current working directory is used.

set -euo pipefail

REPO="giulio-leone/workflow-istrunctions"
TARGET="${1:-$(pwd)}"
TARGET="$(cd "$TARGET" && pwd)"

echo "📥  Syncing workflow instructions into: $TARGET"

# Prefer gh CLI; fall back to curl+token
if command -v gh &>/dev/null && gh auth status &>/dev/null 2>&1; then
  _download() { gh api "repos/$REPO/contents/$1" --jq '.content' | base64 -d; }
  _list_tree() { gh api "repos/$REPO/git/trees/main?recursive=1" --jq '.tree[].path'; }
else
  if [ -z "${GITHUB_TOKEN:-}" ]; then
    echo "❌  Neither 'gh' CLI (authenticated) nor GITHUB_TOKEN is available."
    echo "    Install gh CLI (https://cli.github.com/) and run 'gh auth login', or set GITHUB_TOKEN."
    exit 1
  fi
  _download() { curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" \
    "https://raw.githubusercontent.com/$REPO/main/$1"; }
  _list_tree() { curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" \
    "https://api.github.com/repos/$REPO/git/trees/main?recursive=1" \
    | python3 -c "import sys,json; [print(i['path']) for i in json.load(sys.stdin)['tree']]"; }
fi

# --- Download dispatcher + runtime-specific AGENTS files ---
echo "  → AGENTS.MD (dispatcher)"
_download "AGENTS.MD" > "$TARGET/AGENTS.MD"

echo "  → AGENTS.vscode.MD"
_download "AGENTS.vscode.MD" > "$TARGET/AGENTS.vscode.MD"

echo "  → AGENTS.copilot-cli.MD"
_download "AGENTS.copilot-cli.MD" > "$TARGET/AGENTS.copilot-cli.MD"

# Backward-compatible lowercase alias (skip on case-insensitive filesystems)
if [ ! -e "$TARGET/AGENTS.md" ]; then
  cp "$TARGET/AGENTS.MD" "$TARGET/AGENTS.md"
fi

# --- Discover and download all SKILL.md files ---
echo "  → Fetching skill list..."
SKILL_PATHS=$(_list_tree | grep '^\.agents/skills/.*/SKILL\.md$')

if [ -z "$SKILL_PATHS" ]; then
  echo "  ⚠️  No SKILL.md files found in .agents/skills/."
  exit 1
fi

COUNT=0
while IFS= read -r path; do
  dest="$TARGET/$path"
  mkdir -p "$(dirname "$dest")"
  echo "  → $path"
  _download "$path" > "$dest"
  COUNT=$((COUNT + 1))
done <<< "$SKILL_PATHS"

echo ""
echo "✅  Sync complete: AGENTS dispatcher + runtime files + $COUNT skill files installed in: $TARGET"
