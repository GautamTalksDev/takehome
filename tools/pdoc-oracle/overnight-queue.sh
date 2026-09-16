#!/usr/bin/env bash
# Overnight PDOC queue worker. Resume is automatic from queue-checkpoint.json.
# Restarts on stall (exit 1) or hard-stop (exit 0 with remaining keys).
set -u
cd "$(dirname "$0")"

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

ROOT="$(cd ../.. && pwd)"
CACHE="$ROOT/data/pdoc-cache"
LOG="$CACHE/queue-run.log"
PROGRESS="$CACHE/queue-progress.log"
CHECKPOINT="$CACHE/queue-checkpoint.json"
QUEUE="$ROOT/data/grids/pdoc-queue.json"

mkdir -p "$CACHE"

completed_n() {
  python3 - "$CHECKPOINT" <<'PY'
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
if not p.exists():
    print(0)
    raise SystemExit
print(len(json.loads(p.read_text())["completedKeys"]))
PY
}

queue_n() {
  python3 - "$QUEUE" <<'PY'
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
if not p.exists():
    print(9762)
    raise SystemExit
print(len(json.loads(p.read_text())))
PY
}

stamp() { date -u +%Y-%m-%dT%H:%M:%SZ; }

echo "$(stamp) overnight loop start target=$(queue_n) completed=$(completed_n)" | tee -a "$PROGRESS"

while true; do
  target="$(queue_n)"
  done_n="$(completed_n)"
  if [ "$done_n" -ge "$target" ]; then
    echo "$(stamp) overnight loop finished completed=$done_n/$target" | tee -a "$PROGRESS"
    break
  fi

  echo "$(stamp) worker start completed=$done_n/$target" | tee -a "$PROGRESS"
  npm run capture:queue >>"$LOG" 2>&1
  code=$?
  done_n="$(completed_n)"
  echo "$(stamp) worker exited $code completed=$done_n/$target" | tee -a "$PROGRESS"
  if [ "$done_n" -ge "$target" ]; then
    echo "$(stamp) overnight loop finished completed=$done_n/$target" | tee -a "$PROGRESS"
    break
  fi
  echo "$(stamp) resuming in 60s" | tee -a "$PROGRESS"
  sleep 60
done
