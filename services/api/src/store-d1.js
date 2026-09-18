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

  async insertWebhook(row) {
    await this.db
      .prepare(
        `INSERT INTO webhook_endpoints (id, account_id, url, secret, created_at)
         VALUES (?, ?, ?, ?, ?)`,
      )
      .bind(row.id, row.account_id, row.url, row.secret, row.created_at)
      .run();
    return row;
  }

  async listWebhooks(accountId) {
    const { results } = await this.db
      .prepare(
        `SELECT id, account_id, url, secret, created_at
         FROM webhook_endpoints WHERE account_id = ? ORDER BY created_at`,
      )
      .bind(accountId)
      .all();
    return results ?? [];
  }

  async getWebhook(id) {
    return (
      (await this.db
        .prepare(
          `SELECT id, account_id, url, secret, created_at
           FROM webhook_endpoints WHERE id = ?`,
        )
        .bind(id)
        .first()) ?? null
    );
  }

  async deleteWebhook(id, accountId) {
    const result = await this.db
      .prepare(`DELETE FROM webhook_endpoints WHERE id = ? AND account_id = ?`)
      .bind(id, accountId)
      .run();
    return (result?.meta?.changes ?? 0) > 0;
  }

  async insertDelivery(row) {
    await this.db
      .prepare(
        `INSERT INTO webhook_deliveries
         (id, endpoint_id, event, payload, status, attempts, last_error,
          last_http_status, replay_of, created_at, delivered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      )
      .bind(
        row.id,
        row.endpoint_id,
        row.event,
        row.payload,
        row.status,
        row.attempts,
        row.last_error ?? null,
        row.last_http_status ?? null,
        row.replay_of ?? null,
        row.created_at,
        row.delivered_at ?? null,
      )
      .run();
    return row;
  }

  async getDelivery(id) {
    return (
      (await this.db
        .prepare(
          `SELECT id, endpoint_id, event, payload, status, attempts, last_error,
                  last_http_status, replay_of, created_at, delivered_at
           FROM webhook_deliveries WHERE id = ?`,
        )
        .bind(id)
        .first()) ?? null
    );
  }

  async listDeliveries(accountId) {
    const { results } = await this.db
      .prepare(
        `SELECT d.id, d.endpoint_id, d.event, d.payload, d.status, d.attempts,
                d.last_error, d.last_http_status, d.replay_of, d.created_at,
                d.delivered_at
         FROM webhook_deliveries d
         JOIN webhook_endpoints e ON e.id = d.endpoint_id
         WHERE e.account_id = ?
         ORDER BY d.created_at DESC`,
      )
      .bind(accountId)
      .all();
    return results ?? [];
  }

  async updateDelivery(id, patch) {
    const row = await this.getDelivery(id);
    if (!row) {
      return null;
    }
    const next = { ...row, ...patch };
    await this.db
      .prepare(
        `UPDATE webhook_deliveries
         SET status = ?, attempts = ?, last_error = ?, last_http_status = ?,
             delivered_at = ?
         WHERE id = ?`,
      )
      .bind(
        next.status,
        next.attempts,
        next.last_error ?? null,
        next.last_http_status ?? null,
        next.delivered_at ?? null,
        id,
      )
      .run();
    return next;
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
