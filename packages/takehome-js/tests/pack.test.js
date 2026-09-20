import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const pkg = JSON.parse(
  readFileSync(
    path.join(path.dirname(fileURLToPath(import.meta.url)), '../package.json'),
    'utf8',
  ),
);

test('56. package.json files is an allowlist of dist, wasm binary, README, LICENSE', () => {
  assert.ok(Array.isArray(pkg.files));
  assert.deepEqual(
    [...pkg.files].sort(),
    ['LICENSE', 'README.md', 'dist', 'wasm/takehome_wasm_bg.wasm'].sort(),
  );
  assert.equal(pkg.files.includes('src'), false);
  assert.equal(pkg.files.includes('tests'), false);
  assert.equal(pkg.files.includes('scripts'), false);
});

test('58. no install-time lifecycle scripts', () => {
  const scripts = pkg.scripts ?? {};
  for (const name of ['preinstall', 'postinstall', 'install', 'prepare', 'preprepare']) {
    assert.equal(scripts[name], undefined, name);
  }
  assert.equal(typeof scripts.prepublishOnly, 'string');
});

test('59. npm publish metadata and provenance', () => {
  assert.equal(pkg.license, 'Apache-2.0');
  assert.equal(pkg.repository?.url, 'git+https://github.com/takehome-ca/takehome.git');
  assert.equal(pkg.repository?.directory, 'packages/takehome-js');
  assert.equal(pkg.homepage, 'https://github.com/takehome-ca/takehome');
  assert.equal(pkg.bugs?.url, 'https://github.com/takehome-ca/takehome/issues');
  assert.equal(pkg.engines?.node, '>=20');
  assert.equal(pkg.publishConfig?.access, 'public');
  assert.equal(pkg.publishConfig?.provenance, true);
});
