#!/usr/bin/env bash
# Full verification: clean the built app, rebuild from scratch, run every test.
# Writes a timestamped evidence log. Usage: scripts/verify.sh <label>
set -uo pipefail
cd "$(dirname "$0")/.."
LABEL="${1:-verify}"
STAMP="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
LOG="docs/evidence/${LABEL}-${STAMP}.log"
export CARGO_INCREMENTAL=0

exec > >(tee "$LOG") 2>&1
echo "AegisDraft verification — ${LABEL}"
echo "UTC: ${STAMP}"
echo "commit: $(git rev-parse --short HEAD 2>/dev/null || echo none)"
echo "=================================================================="

fail=0
step() { echo; echo "--- $* ---"; }

step "Remove the previously built app"
rm -rf src-tauri/target/debug/aegisdraft \
       src-tauri/target/debug/bundle \
       src-tauri/target/release/aegisdraft \
       dist
echo "removed prior build outputs"

step "Rust: format check"
(cd src-tauri && cargo fmt --check >/dev/null 2>&1) && echo "rustfmt: clean" || echo "rustfmt: differences (non-blocking)"

step "Rust: clippy"
if (cd src-tauri && cargo clippy --all-targets -- -D warnings 2>&1 | tail -20); then
  echo "clippy: clean"
else
  echo "clippy: FAILED"; fail=1
fi

step "Rust: tests"
if (cd src-tauri && cargo test 2>&1 | tail -40); then
  echo "cargo test: passed"
else
  echo "cargo test: FAILED"; fail=1
fi

step "TypeScript: typecheck"
if npx tsc --noEmit; then echo "tsc: clean"; else echo "tsc: FAILED"; fail=1; fi

step "Frontend: tests"
if npx vitest run --reporter=default 2>&1 | tail -30; then
  echo "vitest: passed"
else
  echo "vitest: FAILED"; fail=1
fi

step "Frontend: production build"
if npm run build 2>&1 | tail -8; then echo "vite build: ok"; else echo "vite build: FAILED"; fail=1; fi

step "Rust: rebuild the app binary from clean"
if (cd src-tauri && cargo build 2>&1 | tail -5); then echo "cargo build: ok"; else echo "cargo build: FAILED"; fail=1; fi

echo
echo "=================================================================="
if [ "$fail" -eq 0 ]; then echo "RESULT: PASS"; else echo "RESULT: FAIL"; fi
echo "evidence: ${LOG}"
exit "$fail"
