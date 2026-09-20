import { DatabaseSync } from 'node:sqlite';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const MIGRATIONS = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '../migrations',
);

export function applyMigrations(db, dir = MIGRATIONS) {
  const files = readdirSync(dir)
    .filter((name) => name.endsWith('.sql'))
    .sort();
  if (files.length === 0) {
    throw new Error(`no migration files in ${dir}`);
  }
  for (const file of files) {
    db.exec(readFileSync(path.join(dir, file), 'utf8'));
  }
  return files;
}

export function openMigratedSqlite(dir = MIGRATIONS) {
  const db = new DatabaseSync(':memory:');
  db.exec('PRAGMA foreign_keys = ON');
  const files = applyMigrations(db, dir);
  return { db, files };
}

function mapAccount(row) {
  return {
    id: row.id,
    email: row.email,
    email_verified: Boolean(row.email_verified),
    plan: row.plan,
    stripe_customer_id: row.stripe_customer_id ?? null,
    created_at: row.created_at,
  };
}

function mapKey(row) {
  if (!row) {
    return null;
  }
  return {
    hash: row.hash,
    account_id: row.account_id,
    kind: row.kind,
    prefix: row.prefix,
    revoked: Boolean(row.revoked),
  };
}

/**
 * Sync SQLite store using the same schema and SQL as D1Store.
 * Used when TAKEHOME_API_STORE=d1 so the API suite can run against migrations.
 */
export class SqliteStore {
  constructor(db) {
    this.db = db;
  }

  static fromMigrations(dir = MIGRATIONS) {
    return new SqliteStore(openMigratedSqlite(dir).db);
  }

  createAccount({ email, email_verified = false, plan = 'developer' }) {
    const id = `acc_${crypto.randomUUID().replaceAll('-', '')}`;
    const created_at = '2026-09-16T00:00:00Z';
    this.db
      .prepare(
        `INSERT INTO accounts (id, email, email_verified, plan, stripe_customer_id, created_at)
         VALUES (?, ?, ?, ?, NULL, ?)`,
      )
      .run(id, email, email_verified ? 1 : 0, plan, created_at);
    return this.getAccount(id);
  }

  getAccount(id) {
    const row = this.db
      .prepare(
        `SELECT id, email, email_verified, plan, stripe_customer_id, created_at
         FROM accounts WHERE id = ?`,
      )
      .get(id);
    return row ? mapAccount(row) : null;
  }

  getAccountByEmail(email) {
    const row = this.db
      .prepare(
        `SELECT id, email, email_verified, plan, stripe_customer_id, created_at
         FROM accounts WHERE email = ?`,
      )
      .get(email);
    return row ? mapAccount(row) : null;
  }

  setPlan(id, plan) {
    this.db.prepare(`UPDATE accounts SET plan = ? WHERE id = ?`).run(plan, id);
  }

  setVerified(id) {
    this.db
      .prepare(`UPDATE accounts SET email_verified = 1 WHERE id = ?`)
      .run(id);
  }

  insertKey({ account_id, kind, prefix, hash, revoked = false }) {
    this.db
      .prepare(
        `INSERT INTO api_keys (hash, account_id, kind, prefix, revoked) VALUES (?, ?, ?, ?, ?)`,
      )
      .run(hash, account_id, kind, prefix, revoked ? 1 : 0);
  }

  getKeyByHash(hash) {
    return mapKey(
      this.db
        .prepare(
          `SELECT hash, account_id, kind, prefix, revoked FROM api_keys WHERE hash = ?`,
        )
        .get(hash),
    );
  }

  listKeys(accountId) {
    return this.db
      .prepare(
        `SELECT hash, account_id, kind, prefix, revoked FROM api_keys WHERE account_id = ?`,
      )
      .all(accountId)
      .map(mapKey);
  }

  revokeKind(accountId, kind) {
    this.db
      .prepare(
        `UPDATE api_keys SET revoked = 1 WHERE account_id = ? AND kind = ?`,
      )
      .run(accountId, kind);
  }

  insertEmailToken({ hash, account_id, expires_at }) {
    this.db
      .prepare(
        `INSERT INTO email_tokens (hash, account_id, expires_at) VALUES (?, ?, ?)`,
      )
      .run(hash, account_id, expires_at);
  }

  getEmailToken(hash) {
    return (
      this.db
        .prepare(
          `SELECT hash, account_id, expires_at FROM email_tokens WHERE hash = ?`,
        )
        .get(hash) ?? null
    );
  }

  deleteEmailToken(hash) {
    this.db.prepare(`DELETE FROM email_tokens WHERE hash = ?`).run(hash);
  }

  deleteEmailTokensForAccount(accountId) {
    this.db
      .prepare(`DELETE FROM email_tokens WHERE account_id = ?`)
      .run(accountId);
  }

  consumeEmailToken(hash) {
    const row = this.getEmailToken(hash);
    if (!row) {
      return null;
    }
    this.deleteEmailToken(hash);
    return row;
  }

  usageKey(accountId, period) {
    return `${accountId}:${period}`;
  }

  getUsage(accountId, period) {
    const row = this.db
      .prepare(
        `SELECT calculations FROM usage WHERE account_id = ? AND period_start = ?`,
      )
      .get(accountId, period);
    return row ? row.calculations : 0;
  }

  setUsage(accountId, period, n) {
    this.db
      .prepare(
        `INSERT INTO usage (account_id, period_start, calculations)
         VALUES (?, ?, ?)
         ON CONFLICT(account_id, period_start) DO UPDATE SET calculations = excluded.calculations`,
      )
      .run(accountId, period, n);
  }

  addUsage(accountId, period, n) {
    const next = this.getUsage(accountId, period) + n;
    this.setUsage(accountId, period, next);
    return next;
  }

  hasStripeEvent(id) {
    const row = this.db
      .prepare(`SELECT id FROM stripe_events WHERE id = ?`)
      .get(id);
    return Boolean(row);
  }

  recordStripeEvent(id) {
    this.db
      .prepare(`INSERT OR IGNORE INTO stripe_events (id) VALUES (?)`)
      .run(id);
  }

  insertWebhook(row) {
    this.db
      .prepare(
        `INSERT INTO webhook_endpoints (id, account_id, url, secret, created_at)
         VALUES (?, ?, ?, ?, ?)`,
      )
      .run(row.id, row.account_id, row.url, row.secret, row.created_at);
    return row;
  }

  listWebhooks(accountId) {
    return this.db
      .prepare(
        `SELECT id, account_id, url, secret, created_at
         FROM webhook_endpoints WHERE account_id = ? ORDER BY created_at`,
      )
      .all(accountId);
  }

  getWebhook(id, accountId) {
    return (
      this.db
        .prepare(
          `SELECT id, account_id, url, secret, created_at
           FROM webhook_endpoints WHERE id = ? AND account_id = ?`,
        )
        .get(id, accountId) ?? null
    );
  }

  deleteWebhook(id, accountId) {
    const result = this.db
      .prepare(`DELETE FROM webhook_endpoints WHERE id = ? AND account_id = ?`)
      .run(id, accountId);
    return result.changes > 0;
  }

  insertDelivery(row) {
    this.db
      .prepare(
        `INSERT INTO webhook_deliveries
         (id, endpoint_id, account_id, event, payload, status, attempts, last_error,
          last_http_status, replay_of, created_at, delivered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      )
      .run(
        row.id,
        row.endpoint_id,
        row.account_id,
        row.event,
        row.payload,
        row.status,
        row.attempts,
        row.last_error ?? null,
        row.last_http_status ?? null,
        row.replay_of ?? null,
        row.created_at,
        row.delivered_at ?? null,
      );
    return row;
  }

  getDelivery(id, accountId) {
    return (
      this.db
        .prepare(
          `SELECT d.id, d.endpoint_id, d.account_id, d.event, d.payload, d.status, d.attempts,
                  d.last_error, d.last_http_status, d.replay_of, d.created_at,
                  d.delivered_at
           FROM webhook_deliveries d
           JOIN webhook_endpoints e ON e.id = d.endpoint_id
           WHERE d.id = ? AND d.account_id = ?`,
        )
        .get(id, accountId) ?? null
    );
  }

  listDeliveries(accountId) {
    return this.db
      .prepare(
        `SELECT d.id, d.endpoint_id, d.account_id, d.event, d.payload, d.status, d.attempts,
                d.last_error, d.last_http_status, d.replay_of, d.created_at,
                d.delivered_at
         FROM webhook_deliveries d
         JOIN webhook_endpoints e ON e.id = d.endpoint_id
         WHERE d.account_id = ?
         ORDER BY d.created_at DESC`,
      )
      .all(accountId);
  }

  updateDelivery(id, patch) {
    const row = this.db
      .prepare(
        `SELECT id, endpoint_id, account_id, event, payload, status, attempts, last_error,
                last_http_status, replay_of, created_at, delivered_at
         FROM webhook_deliveries WHERE id = ?`,
      )
      .get(id);
    if (!row) {
      return null;
    }
    const next = { ...row, ...patch };
    this.db
      .prepare(
        `UPDATE webhook_deliveries
         SET status = ?, attempts = ?, last_error = ?, last_http_status = ?,
             delivered_at = ?
         WHERE id = ?`,
      )
      .run(
        next.status,
        next.attempts,
        next.last_error ?? null,
        next.last_http_status ?? null,
        next.delivered_at ?? null,
        id,
      );
    return next;
  }

  dump() {
    return {
      api_keys: this.db
        .prepare(
          `SELECT hash, kind, prefix, account_id, revoked FROM api_keys`,
        )
        .all()
        .map(mapKey),
      accounts: this.db
        .prepare(
          `SELECT id, email, email_verified, plan, stripe_customer_id, created_at FROM accounts`,
        )
        .all()
        .map(mapAccount),
    };
  }

  get accounts() {
    const map = new Map();
    for (const row of this.dump().accounts) {
      map.set(row.id, row);
    }
    return map;
  }

  get keys() {
    const map = new Map();
    for (const row of this.db
      .prepare(
        `SELECT hash, account_id, kind, prefix, revoked FROM api_keys`,
      )
      .all()
      .map(mapKey)) {
      map.set(row.hash, row);
    }
    return map;
  }

  get usage() {
    const map = new Map();
    for (const row of this.db
      .prepare(`SELECT account_id, period_start, calculations FROM usage`)
      .all()) {
      map.set(this.usageKey(row.account_id, row.period_start), row.calculations);
    }
    return map;
  }

  get deliveries() {
    const map = new Map();
    for (const row of this.db
      .prepare(
        `SELECT id, endpoint_id, account_id, event, payload, status, attempts, last_error,
                last_http_status, replay_of, created_at, delivered_at
         FROM webhook_deliveries`,
      )
      .all()) {
      map.set(row.id, row);
    }
    return map;
  }
}

export function newStore() {
  if (process.env.TAKEHOME_API_STORE === 'd1') {
    return SqliteStore.fromMigrations();
  }
  return null;
}
