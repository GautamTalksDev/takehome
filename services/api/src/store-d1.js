export class D1Store {
  constructor(db) {
    this.db = db;
  }

  async createAccount({ email, email_verified = false, plan = 'developer' }) {
    const id = `acc_${crypto.randomUUID().replaceAll('-', '')}`;
    const created_at = '2026-09-16T00:00:00Z';
    await this.db
      .prepare(
        `INSERT INTO accounts (id, email, email_verified, plan, stripe_customer_id, created_at)
         VALUES (?, ?, ?, ?, NULL, ?)`,
      )
      .bind(id, email, email_verified ? 1 : 0, plan, created_at)
      .run();
    return this.getAccount(id);
  }

  async getAccount(id) {
    const row = await this.db
      .prepare(
        `SELECT id, email, email_verified, plan, stripe_customer_id, created_at
         FROM accounts WHERE id = ?`,
      )
      .bind(id)
      .first();
    return row ? mapAccount(row) : null;
  }

  async getAccountByEmail(email) {
    const row = await this.db
      .prepare(
        `SELECT id, email, email_verified, plan, stripe_customer_id, created_at
         FROM accounts WHERE email = ?`,
      )
      .bind(email)
      .first();
    return row ? mapAccount(row) : null;
  }

  async setPlan(id, plan) {
    await this.db
      .prepare(`UPDATE accounts SET plan = ? WHERE id = ?`)
      .bind(plan, id)
      .run();
  }

  async setVerified(id) {
    await this.db
      .prepare(`UPDATE accounts SET email_verified = 1 WHERE id = ?`)
      .bind(id)
      .run();
  }

  async insertKey({ account_id, kind, prefix, hash }) {
    await this.db
      .prepare(
        `INSERT INTO api_keys (hash, account_id, kind, prefix) VALUES (?, ?, ?, ?)`,
      )
      .bind(hash, account_id, kind, prefix)
      .run();
  }

  async getKeyByHash(hash) {
    const row = await this.db
      .prepare(
        `SELECT hash, account_id, kind, prefix FROM api_keys WHERE hash = ?`,
      )
      .bind(hash)
      .first();
    return row ?? null;
  }

  async insertEmailToken({ hash, account_id, expires_at }) {
    await this.db
      .prepare(
        `INSERT INTO email_tokens (hash, account_id, expires_at) VALUES (?, ?, ?)`,
      )
      .bind(hash, account_id, expires_at)
      .run();
  }

  async consumeEmailToken(hash) {
    const row = await this.db
      .prepare(
        `SELECT hash, account_id, expires_at FROM email_tokens WHERE hash = ?`,
      )
      .bind(hash)
      .first();
    if (!row) {
      return null;
    }
    await this.db
      .prepare(`DELETE FROM email_tokens WHERE hash = ?`)
      .bind(hash)
      .run();
    return row;
  }

  async getUsage(accountId, period) {
    const row = await this.db
      .prepare(
        `SELECT calculations FROM usage WHERE account_id = ? AND period_start = ?`,
      )
      .bind(accountId, period)
      .first();
    return row ? row.calculations : 0;
  }

  async setUsage(accountId, period, n) {
    await this.db
      .prepare(
        `INSERT INTO usage (account_id, period_start, calculations)
         VALUES (?, ?, ?)
         ON CONFLICT(account_id, period_start) DO UPDATE SET calculations = excluded.calculations`,
      )
      .bind(accountId, period, n)
      .run();
  }

  async addUsage(accountId, period, n) {
    const used = await this.getUsage(accountId, period);
    const next = used + n;
    await this.setUsage(accountId, period, next);
    return next;
  }

  async hasStripeEvent(id) {
    const row = await this.db
      .prepare(`SELECT id FROM stripe_events WHERE id = ?`)
      .bind(id)
      .first();
    return Boolean(row);
  }

  async recordStripeEvent(id) {
    await this.db
      .prepare(`INSERT OR IGNORE INTO stripe_events (id) VALUES (?)`)
      .bind(id)
      .run();
  }
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
