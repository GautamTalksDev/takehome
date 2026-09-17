#!/usr/bin/env bash
# Takehome M0 scaffold. Run from the directory where you want `takehome/` to live.
# Assumes: rustup, cargo, node >=20, git, gh (optional). WSL2/Ubuntu or macOS/Linux.
set -euo pipefail

ORG=takehome-ca
REPO=takehome

# ---------------------------------------------------------------- 0. toolchain
rustup toolchain install 1.90.0 >/dev/null 2>&1 || true
rustup default 1.90.0
rustup component add clippy rustfmt
cargo install cargo-deny --locked >/dev/null 2>&1 || true

# ---------------------------------------------------------------- 1. tree
mkdir -p "$REPO" && cd "$REPO"
git init -q

mkdir -p \
  crates/takehome-core/src/{rules,formulas/province} \
  crates/takehome-core/tests/vectors \
  crates/takehome-wasm/src \
  crates/takehome-cli/src \
  packages/takehome-js \
  packages/takehome-py \
  packages/takehome-go \
  services/api/src/handlers \
  web/site/src/{pages/calculators,pages/docs,pages/changes,pages/conformance,components,engine} \
  data/rules/2026-01-01 data/rules/2026-07-01 data/rules/2027-01-01 \
  data/sources data/changelog \
  tools/{ingest,pdoc-oracle,grid-gen,differential} \
  docs \
  .github/workflows

# ---------------------------------------------------------------- 2. toolchain pin
cat > rust-toolchain.toml <<'EOF'
[toolchain]
channel = "1.90.0"
components = ["clippy", "rustfmt"]
EOF

cat > rustfmt.toml <<'EOF'
edition = "2021"
max_width = 100
EOF

# ---------------------------------------------------------------- 3. workspace
cat > Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = ["crates/takehome-core", "crates/takehome-wasm", "crates/takehome-cli"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/takehome-ca/takehome"
rust-version = "1.90"

[workspace.dependencies]
rust_decimal = { version = "1", default-features = false, features = ["serde-with-str", "maths"] }
rust_decimal_macros = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
proptest = "1"
EOF

cat > crates/takehome-core/Cargo.toml <<'EOF'
[package]
name = "takehome-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Deterministic implementation of the CRA T4127 payroll deduction formulas."

[dependencies]
rust_decimal.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
rust_decimal_macros.workspace = true
proptest.workspace = true

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
float_arithmetic = "deny"
cast_possible_truncation = "deny"
cast_precision_loss = "deny"
cast_sign_loss = "deny"
EOF

# placeholder crates so the workspace builds green today
for c in takehome-wasm takehome-cli; do
cat > "crates/$c/Cargo.toml" <<EOF
[package]
name = "$c"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
takehome-core = { path = "../takehome-core" }
EOF
done
echo 'pub fn placeholder() {}' > crates/takehome-wasm/src/lib.rs
printf 'fn main() {\n    println!("takehome cli placeholder");\n}\n' > crates/takehome-cli/src/main.rs

# ---------------------------------------------------------------- 4. core module skeleton
cat > crates/takehome-core/src/lib.rs <<'EOF'
//! takehome-core: no IO, no network, no clock, no environment access.
//! Given a Request and a RuleSet, returns a Response. Deterministic, forever.
#![forbid(unsafe_code)]

pub mod decimal;
pub mod rounding;
EOF

: > crates/takehome-core/src/decimal.rs
: > crates/takehome-core/src/rounding.rs

# ---------------------------------------------------------------- 5. governance files
cat > LICENSE-APACHE <<'EOF'
Apache License 2.0 - full text: https://www.apache.org/licenses/LICENSE-2.0.txt
(Replace this stub with the full text before the first public commit.)
EOF

cat > KILL-TEST.md <<'EOF'
# PRE-REGISTERED STOP CONDITION

Written before any engine code was committed. Commit date is the record.

Within 90 days of the API going live, at least ONE of:

  A. Twenty-five distinct domains have made a successful, non-test,
     authenticated API call, excluding CI and excluding anyone I
     personally know.

  B. The calculator pages record three thousand organic search
     sessions in a single calendar month.

  C. Three paying customers on any tier above free.

If none of the three fire, the numbers are published as they stand and
the hosted service is shut down. The engine and the rule data remain
published under their open licences.

API live date: <FILL IN>
Day 90: <FILL IN>
EOF

cat > README.md <<'EOF'
# Takehome

Canadian payroll deduction calculation. Effective-date versioned. Apache-2.0.

```bash
curl -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -d '{"province":"ON","pay_period":26,"gross_pay":"2500.00"}'
```

Every response names the rule set version that answered it. Ask for a date in
February 2026 in November 2026 and you get February's rules.

- Conformance record (including every disagreement with CRA PDOC): /conformance
- Machine-readable change log: /changes
- Pre-registered stop condition: [KILL-TEST.md](./KILL-TEST.md)

Takehome is a calculation service, not a payroll provider. It does not file
returns or move money. The CRA's Payroll Deductions Online Calculator is the
authoritative source.
EOF

cat > CONFORMANCE.md <<'EOF'
# Conformance

Regenerated by CI. Not yet run.

## Agreement rate
Not yet measured.

## Disagreements
None recorded yet. This section exists before there is anything to put in it,
and every case found will be listed here with full input, both outputs, the
delta, and an explanation.

## Coverage gaps
Everything. M0 covers arithmetic primitives only.
EOF

cat > CHANGELOG.md <<'EOF'
# Changelog

## [Unreleased]
- M0: fixed-point decimal type and CRA rounding rules.
EOF

cat > .gitignore <<'EOF'
target/
node_modules/
dist/
.wrangler/
*.log
.DS_Store
data/sources/**/*.html
!data/sources/.gitkeep
EOF

touch data/sources/.gitkeep data/changelog/.gitkeep
echo '[]' > data/changelog/changes.json

# ---------------------------------------------------------------- 6. float ban
mkdir -p scripts
cat > scripts/float-ban.sh <<'EOF'
#!/usr/bin/env bash
# Fails if IEEE-754 types appear anywhere in the money path.
set -euo pipefail
PATHS=("crates/takehome-core/src")
PATTERN='\b(f32|f64)\b|\bas f(32|64)\b|to_f64|to_f32|from_f64|from_f32|parse::<f(32|64)>'
FOUND=0
for p in "${PATHS[@]}"; do
  if grep -rInE --include='*.rs' "$PATTERN" "$p"; then
    FOUND=1
  fi
done
if [ "$FOUND" -eq 1 ]; then
  echo "FLOAT BAN VIOLATION: binary floating point found in a money path." >&2
  exit 1
fi
echo "float-ban: clean"
EOF
chmod +x scripts/float-ban.sh scaffold.sh 2>/dev/null || true

# ---------------------------------------------------------------- 7. first commit
git add -A
git commit -q -m "M0: scaffold, kill test, float ban"
echo
echo "Scaffold complete. Next:"
echo "  gh repo create $ORG/$REPO --public --source=. --push   # if you use gh"
echo "  cargo test --workspace"
