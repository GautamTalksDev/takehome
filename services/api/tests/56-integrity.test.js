/**
 * A08:2025 Software or Data Integrity Failures — items 45–47.
 */
import assert from 'node:assert/strict';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { embedSri, embedSnippet, EMBED_PATH } from '../../../web/site/src/lib/embed-sri.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '../../..');
const CORE = path.join(REPO, 'crates/takehome-core/src/rules');
const LOADER = readFileSync(path.join(CORE, 'loader.rs'), 'utf8');
const SIGNATURE = readFileSync(path.join(CORE, 'signature.rs'), 'utf8');
const SIGS = JSON.parse(
  readFileSync(path.join(REPO, 'data/rules/signatures.json'), 'utf8'),
);
const TEST_YML = readFileSync(path.join(REPO, '.github/workflows/test.yml'), 'utf8');
const PUBLISH_YML = readFileSync(
  path.join(REPO, '.github/workflows/publish.yml'),
  'utf8',
);
const CHECK = readFileSync(path.join(REPO, 'scripts/check-wasm-hash.sh'), 'utf8');
const CHECK_BIN = readFileSync(
  path.join(REPO, 'scripts/check-binaryen-version.sh'),
  'utf8',
);
const INSTALL_BIN = readFileSync(
  path.join(REPO, 'scripts/install-binaryen.sh'),
  'utf8',
);
const BUILD_SH = readFileSync(
  path.join(REPO, 'packages/takehome-js/scripts/build.sh'),
  'utf8',
);
const PIN_PATH = path.join(REPO, 'packages/takehome-js/binaryen-version');
const CLAIM_PATH = path.join(REPO, 'packages/takehome-js/wasm.sha256');
const HEADERS = readFileSync(path.join(REPO, 'web/site/public/_headers'), 'utf8');
const EMBED_JS = readFileSync(path.join(REPO, 'web/site/public/embed.js'), 'utf8');
const EMBED_PAGE = readFileSync(
  path.join(REPO, 'web/site/src/pages/embed.astro'),
  'utf8',
);
const DOCS_PAGE = readFileSync(
  path.join(REPO, 'web/site/src/pages/docs.astro'),
  'utf8',
);
const VERSIONED_EMBED = path.join(
  REPO,
  'web/site/public/embed/v0.1.0/embed.js',
);

test('45. loader verifies Ed25519 signatures of embedded rule-set files', () => {
  assert.match(SIGNATURE, /ed25519/i);
  assert.match(SIGNATURE, /spec §M5|§M5 self-host/);
  assert.match(SIGNATURE, /pub fn verify_rules_file/);
  assert.match(LOADER, /verify_rules_file/);
  assert.equal(SIGS.algorithm, 'ed25519');
  assert.match(SIGS.public_key, /^[0-9a-f]{64}$/);
  const files = Object.keys(SIGS.files);
  assert.ok(files.length >= 30, `expected every embedded file, got ${files.length}`);
  for (const rel of files) {
    assert.match(SIGS.files[rel], /^[0-9a-f]{128}$/, rel);
    const onDisk = readFileSync(path.join(REPO, 'data/rules', rel));
    const msg = Buffer.concat([
      Buffer.from(rel, 'utf8'),
      Buffer.from([0]),
      onDisk,
    ]);
    const key = createPublicKey({
      key: Buffer.concat([
        Buffer.from('302a300506032b6570032100', 'hex'),
        Buffer.from(SIGS.public_key, 'hex'),
      ]),
      format: 'der',
      type: 'spki',
    });
    assert.equal(
      verify(null, msg, key, Buffer.from(SIGS.files[rel], 'hex')),
      true,
      rel,
    );
  }
  const tampered = Buffer.from(readFileSync(path.join(REPO, 'data/rules', files[0])));
  tampered[0] ^= 0xff;
  const msg = Buffer.concat([
    Buffer.from(files[0], 'utf8'),
    Buffer.from([0]),
    tampered,
  ]);
  const key = createPublicKey({
    key: Buffer.concat([
      Buffer.from('302a300506032b6570032100', 'hex'),
      Buffer.from(SIGS.public_key, 'hex'),
    ]),
    format: 'der',
    type: 'spki',
  });
  assert.equal(
    verify(null, msg, key, Buffer.from(SIGS.files[files[0]], 'hex')),
    false,
  );
});

test('46. embed.js is pinned on an immutable versioned path with an SRI hash', () => {
  assert.equal(
    readFileSync(VERSIONED_EMBED).equals(Buffer.from(EMBED_JS)),
    true,
    'versioned embed.js must be byte-identical to /embed.js',
  );
  assert.match(
    HEADERS,
    /\/embed\/v0\.1\.0\/embed\.js[\s\S]*?Cache-Control:\s*public,\s*max-age=31536000,\s*immutable/,
  );
  assert.match(EMBED_JS, /\.origin\s*\+/);
  const sri = `sha384-${createHash('sha384').update(EMBED_JS).digest('base64')}`;
  assert.equal(EMBED_PATH, '/embed/v0.1.0/embed.js');
  assert.equal(embedSri(), sri);
  assert.equal(embedSnippet().includes(sri), true);
  assert.equal(embedSnippet().includes('crossorigin="anonymous"'), true);
  assert.match(EMBED_PAGE, /embedSri/);
  assert.match(EMBED_PAGE, /EMBED_PATH/);
  assert.match(EMBED_PAGE, /crossorigin="anonymous"/);
  assert.match(DOCS_PAGE, /embedSnippet/);
  assert.match(DOCS_PAGE, /embedSri/);
});

test('47. CI fails when the built WASM hash differs from the release claim', () => {
  assert.equal(existsSync(CLAIM_PATH), true, 'packages/takehome-js/wasm.sha256');
  const claimed = readFileSync(CLAIM_PATH, 'utf8');
  assert.match(claimed, /^[0-9a-f]{64} {2}takehome_wasm_bg\.wasm\n?$/);
  assert.match(CHECK, /sha256sum|createHash\('sha256'\)/);
  assert.match(CHECK, /exit 1/);
  assert.match(BUILD_SH, /remap-path-prefix/);
  assert.match(TEST_YML, /check-wasm-hash\.sh/);
  assert.match(PUBLISH_YML, /check-wasm-hash\.sh/);
  assert.match(TEST_YML, /check-wasm-paths\.sh/);
  assert.match(PUBLISH_YML, /check-wasm-paths\.sh/);
  const PATHS = readFileSync(path.join(REPO, 'scripts/check-wasm-paths.sh'), 'utf8');
  assert.match(PATHS, /gautamtalksdev/);
  assert.match(PATHS, /\/home\//);
  assert.match(PATHS, /\/\.cargo\//);
  const wasm = path.join(REPO, 'packages/takehome-js/wasm/takehome_wasm_bg.wasm');
  if (existsSync(wasm)) {
    const got = createHash('sha256').update(readFileSync(wasm)).digest('hex');
    const want = claimed.trim().split(/\s+/)[0];
    assert.equal(got, want, 'built WASM must match packages/takehome-js/wasm.sha256');
  }
});

test('47b. wasm-opt is pinned Binaryen, not an optional WASM_OPT skip', () => {
  assert.equal(existsSync(PIN_PATH), true, 'packages/takehome-js/binaryen-version');
  const pin = readFileSync(PIN_PATH, 'utf8').trim();
  assert.match(pin, /^\d+$/, 'binaryen-version must be a numeric Binaryen tag');
  assert.match(BUILD_SH, /\$WASM_OPT" -Oz/);
  assert.match(BUILD_SH, /--enable-bulk-memory-opt/);
  assert.match(BUILD_SH, /check-binaryen-version/);
  assert.match(BUILD_SH, /install-binaryen/);
  assert.match(BUILD_SH, /binaryen-version/);
  assert.doesNotMatch(BUILD_SH, /\$\{WASM_OPT:-/);
  assert.doesNotMatch(BUILD_SH, /WASM_OPT=1/);
  assert.match(CHECK_BIN, /binaryen-version/);
  assert.match(CHECK_BIN, /exit 1/);
  assert.match(INSTALL_BIN, /binaryen-version_/);
  assert.match(TEST_YML, /check-binaryen-version\.sh/);
  assert.match(PUBLISH_YML, /check-binaryen-version\.sh/);
  const ciPins = [
    ...TEST_YML.matchAll(/BINARYEN_VERSION:\s*"(\d+)"/g),
    ...PUBLISH_YML.matchAll(/BINARYEN_VERSION:\s*"(\d+)"/g),
  ].map((row) => row[1]);
  assert.ok(ciPins.length >= 2, 'CI must record BINARYEN_VERSION in test.yml and publish.yml');
  for (const recorded of ciPins) {
    assert.equal(
      recorded,
      pin,
      `CI BINARYEN_VERSION ${recorded} drifted from packages/takehome-js/binaryen-version ${pin}`,
    );
  }
  const drifted = TEST_YML.replace(
    `BINARYEN_VERSION: "${pin}"`,
    'BINARYEN_VERSION: "0"',
  );
  const driftedPins = [...drifted.matchAll(/BINARYEN_VERSION:\s*"(\d+)"/g)].map(
    (row) => row[1],
  );
  assert.ok(
    driftedPins.some((p) => p !== pin),
    'pin comparison must fail if CI restates a different Binaryen version',
  );
});
