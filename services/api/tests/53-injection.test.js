/**
 * A05:2025 Injection — items 32 (D1) and 35 (signup email headers).
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, BASE_ENV, newStore } from './helpers.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '../../..');
const STORE = readFileSync(path.join(HERE, '../src/store-d1.js'), 'utf8');
const WORKFLOW = readFileSync(
  path.join(REPO, '.github/workflows/test.yml'),
  'utf8',
);

test('32. every D1 query uses prepared statements with bound parameters', () => {
  const prepares = [...STORE.matchAll(/\.prepare\(\s*`([\s\S]*?)`/g)];
  assert.ok(prepares.length >= 10, `expected D1 prepares, found ${prepares.length}`);
  for (const match of prepares) {
    const sql = match[1];
    assert.doesNotMatch(sql, /\$\{/, `interpolated SQL: ${sql.slice(0, 80)}`);
    assert.match(sql, /\?/, `unparameterized SQL: ${sql.slice(0, 80)}`);
  }
  assert.equal(
    (STORE.match(/\.prepare\(/g) || []).length,
    (STORE.match(/\.bind\(/g) || []).length,
    'every prepare() must have a matching bind()',
  );
  assert.match(WORKFLOW, /sql-ban\.sh/);

  const ban = spawnSync('bash', [path.join(REPO, 'scripts/sql-ban.sh')], {
    encoding: 'utf8',
  });
  assert.equal(ban.status, 0, ban.stdout + ban.stderr);
});

test('35. signup email rejects CR/LF header injection', async () => {
  const payloads = [
    'ada@example.com\r\nBcc: evil@x.com',
    'ada@example.com\nCc: evil@x.com',
    'ada@example.com\rTo: victim@x.com',
    '\rada@example.com',
    'ada@example.com\0evil@x.com',
  ];
  for (const email of payloads) {
    const mailbox = [];
    const world = {
      env: { ...BASE_ENV, STORE: newStore(), MAILBOX: mailbox },
    };
    const { status, json } = await call(
      'POST',
      '/v1/signup',
      { email },
      {},
      world,
    );
    assert.equal(status, 400, JSON.stringify({ email, json }));
    assert.equal(json.error.code, 'invalid_request');
    assert.equal(mailbox.length, 0, `mail sent for ${JSON.stringify(email)}`);
  }

  const mailbox = [];
  const world = {
    env: { ...BASE_ENV, STORE: newStore(), MAILBOX: mailbox },
  };
  const form = await call(
    'POST',
    '/v1/signup',
    'email=ada@example.com%0d%0aBcc:+evil@x.com',
    { 'content-type': 'application/x-www-form-urlencoded' },
    world,
  );
  assert.equal(form.status, 400, form.text);
  assert.equal(mailbox.length, 0);
});
