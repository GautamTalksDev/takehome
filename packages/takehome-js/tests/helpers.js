import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '../../..');

export function repoPath(...parts) {
  return path.join(REPO, ...parts);
}

export function wasmPath() {
  return path.join(HERE, '../wasm/takehome_wasm_bg.wasm');
}

export function nativeCli() {
  const fromEnv = process.env.TAKEHOME_CLI;
  if (fromEnv) {
    return path.resolve(fromEnv);
  }
  const release = repoPath('target/release/takehome');
  const debug = repoPath('target/debug/takehome');
  const releaseAlias = repoPath('target/release/takehome-cli');
  const debugAlias = repoPath('target/debug/takehome-cli');
  for (const candidate of [release, debug, releaseAlias, debugAlias]) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(
    'native takehome CLI not built; run cargo build -p takehome-cli (or set TAKEHOME_CLI)',
  );
}

export async function loadEngine() {
  const mod = await import('../src/node.js');
  await mod.init();
  return mod;
}

export function loadM1Vectors() {
  const file = repoPath(
    'crates/takehome-core/tests/vectors/pdoc_ontario_2026_01.json',
  );
  const parsed = JSON.parse(readFileSync(file, 'utf8'));
  assert.equal(parsed.vectors.length, 20, 'M1 corpus is twenty vectors');
  return parsed.vectors;
}

export function loadGridSample() {
  const file = path.join(HERE, 'fixtures/grid-sample-500.json');
  const parsed = JSON.parse(readFileSync(file, 'utf8'));
  assert.equal(parsed.requests.length, 500, 'grid sample must be 500 requests');
  return parsed;
}

export function nativeCalculateBatch(requests) {
  const cli = nativeCli();
  const input = JSON.stringify(requests);
  const result = spawnSync(cli, ['calculate-batch'], {
    input,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  });
  if (result.error) {
    throw new Error(`native calculate-batch spawn failed (${cli}): ${result.error}`);
  }
  if (result.status !== 0) {
    throw new Error(
      `native calculate-batch failed (${result.status}): ${result.stderr}`,
    );
  }
  return JSON.parse(result.stdout);
}
