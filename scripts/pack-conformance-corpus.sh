#!/usr/bin/env bash
# Deterministic PDOC corpus archive for the GitHub release asset.
# Does not publish. Does not add records/ to git.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CACHE="$ROOT/data/pdoc-cache"
NAME="takehome-conformance-corpus-2026.1.tar.zst"
ARCHIVE="$CACHE/$NAME"
META="$CACHE/corpus-archive.json"
DOWNLOAD="https://github.com/takehome-ca/takehome/releases/download/conformance-corpus-2026.1/${NAME}"

for need in records pdoc-identity.json screenshot-hashes.json; do
  if [[ ! -e "$CACHE/$need" ]]; then
    echo "pack-conformance-corpus: missing $CACHE/$need" >&2
    exit 1
  fi
done

tmp="$(mktemp "${TMPDIR:-/tmp}/takehome-corpus.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
tar --sort=name \
  --mtime='2026-07-01T00:00:00Z' \
  --owner=0 --group=0 --numeric-owner \
  --pax-option='exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime' \
  -C "$CACHE" \
  -cf - \
  records pdoc-identity.json screenshot-hashes.json \
  | zstd -19 -T1 -q -f -o "$tmp"
mv -f "$tmp" "$ARCHIVE"
trap - EXIT

sha="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
catalog="$(python3 - <<PY
import hashlib, json
from pathlib import Path
root = Path(r"$CACHE") / "records"
hashes = {}
for path in root.glob("*.json"):
    rec = json.loads(path.read_text())
    hashes[rec["key"]] = rec["screenshotSha256"]
canonical = "".join(f"{k} {hashes[k]}\n" for k in sorted(hashes))
print(hashlib.sha256(canonical.encode()).hexdigest())
PY
)"

python3 - <<PY
import json
from pathlib import Path
meta = {
    "filename": "$NAME",
    "sha256": "$sha",
    "catalog_digest": "$catalog",
    "download": "$DOWNLOAD",
}
Path(r"$META").write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n")
print(json.dumps({"wrote": "$ARCHIVE", **meta}))
PY
