#!/usr/bin/env bash
# Launches the app once per screen and captures it, so every screen can be
# reviewed against a real project. Needs Screen Recording permission.
# Usage: scripts/capture-screens.sh <project.aegis> <outdir> [routes...]
set -uo pipefail
cd "$(dirname "$0")/.."
PROJECT="$1"; OUT="$2"; shift 2
ROUTES=("$@")
[ ${#ROUTES[@]} -eq 0 ] && ROUTES=(studio integrity story insights settings)
mkdir -p "$OUT"

for route in "${ROUTES[@]}"; do
  pkill -f "target/debug/aegisdraft" 2>/dev/null; sleep 2
  AEGISDRAFT_OPEN="$PROJECT" AEGISDRAFT_ROUTE="$route" npx tauri dev --no-watch >/dev/null 2>&1 &
  for _ in $(seq 1 60); do pgrep -f "target/debug/aegisdraft" >/dev/null && break; sleep 1; done
  sleep 9
  front=$(osascript -e 'tell application "System Events" to return name of first process whose frontmost is true' 2>/dev/null)
  screencapture -x -o "$OUT/$route.png"
  echo "$route: captured (frontmost=$front)"
done
pkill -f "target/debug/aegisdraft" 2>/dev/null
