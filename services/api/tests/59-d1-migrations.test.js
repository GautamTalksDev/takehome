/**
 * D1 schema is versioned under migrations/. Apply locally only. Never --remote.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readdirSync, readFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { openMigratedSqlite } from './store-sqlite.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const API = path.resolve(HERE, '..');
const MIGRATIONS = path.join(API, 'migrations');
const WRANGLER = readFileSync(path.join(API, 'wrangler.toml'), 'utf8');

test('59. migrations are versioned SQL with account_id indexes', () => {
  const files = readdirSync(MIGRATIONS)
    .filter((name) => name.endsWith('.sql'))
    .sort();
  assert.deepEqual(files, [
    '0001_init.sql',
    '0002_webhooks.sql',
    '0003_key_revocation.sql',
    '0004_account_id_indexes.sql',
  ]);
  assert.match(WRANGLER, /migrations_dir\s*=\s*"migrations"/);
  assert.doesNotMatch(WRANGLER, /--remote/);

  const init = readFileSync(path.join(MIGRATIONS, '0001_init.sql'), 'utf8');
  for (const table of ['api_keys', 'email_tokens', 'usage']) {
    assert.match(
      init,
      new RegExp(`CREATE INDEX idx_${table}_account_id ON ${table}\\(account_id\\)`),
    );
  }

  const webhooks = readFileSync(path.join(MIGRATIONS, '0002_webhooks.sql'), 'utf8');
  assert.match(webhooks, /CREATE TABLE webhook_deliveries/);
  assert.match(webhooks, /account_id TEXT NOT NULL/);
  assert.match(
    webhooks,
    /CREATE INDEX idx_webhook_endpoints_account_id ON webhook_endpoints\(account_id\)/,
  );
  assert.match(
    webhooks,
    /CREATE INDEX idx_webhook_deliveries_account_id ON webhook_deliveries\(account_id\)/,
  );
});

test('59. applying migrations produces the ownership schema', () => {
  const { db, files } = openMigratedSqlite(MIGRATIONS);
  assert.equal(files.length, 4);
  const tables = db
    .prepare(`SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name`)
    .all()
    .map((row) => row.name);
  assert.deepEqual(
    tables.filter((name) => !name.startsWith('sqlite_')),
    [
      'accounts',
      'api_keys',
      'email_tokens',
      'stripe_events',
      'usage',
      'webhook_deliveries',
      'webhook_endpoints',
    ],
  );
  const indexes = db
    .prepare(
      `SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_%' ORDER BY name`,
    )
    .all()
    .map((row) => row.name);
  for (const name of [
    'idx_api_keys_account_id',
    'idx_email_tokens_account_id',
    'idx_usage_account_id',
    'idx_webhook_endpoints_account_id',
    'idx_webhook_deliveries_account_id',
  ]) {
    assert.ok(indexes.includes(name), name);
  }
  const cols = db
    .prepare(`PRAGMA table_info(webhook_deliveries)`)
    .all()
    .map((row) => row.name);
  assert.ok(cols.includes('account_id'));
});

test('59. wrangler d1 execute --local applies migrations (not remote)', () => {
  const wrangler = path.join(API, 'node_modules/.bin/wrangler');
  const state = path.join(API, '.wrangler');
  rmSync(state, { recursive: true, force: true });
  const files = ['0001_init.sql', '0002_webhooks.sql', '0003_key_revocation.sql'];
  for (const file of files) {
    const args = [
      'd1',
      'execute',
      'takehome',
      '--local',
      '--file',
      path.join(MIGRATIONS, file),
    ];
    assert.equal(args.includes('--remote'), false);
    const result = spawnSync(wrangler, args, { cwd: API, encoding: 'utf8' });
    assert.equal(
      result.status,
      0,
      `${file}: ${result.stdout}\n${result.stderr}`,
    );
    const out = `${result.stdout}\n${result.stderr}`;
    assert.match(out, /local database/);
  }
});
