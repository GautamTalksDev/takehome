export class MemoryStore {
  constructor() {
    this.accounts = new Map();
    this.keys = new Map();
    this.tokens = new Map();
    this.usage = new Map();
    this.stripeEvents = new Set();
    this.webhooks = new Map();
    this.deliveries = new Map();
  }

  createAccount({ email, email_verified = false, plan = 'developer' }) {
    const account = {
      id: `acc_${this.accounts.size + 1}`,
      email,
      email_verified: Boolean(email_verified),
      plan,
      stripe_customer_id: null,
      created_at: '2026-09-16T00:00:00Z',
    };
    this.accounts.set(account.id, account);
    return account;
  }

  getAccount(id) {
    return this.accounts.get(id) ?? null;
  }

  getAccountByEmail(email) {
    return (
      [...this.accounts.values()].find((row) => row.email === email) ?? null
    );
  }

  setPlan(id, plan) {
    const account = this.accounts.get(id);
    if (account) {
      account.plan = plan;
    }
  }

  setVerified(id) {
    const account = this.accounts.get(id);
    if (account) {
      account.email_verified = true;
    }
  }

  insertKey({ account_id, kind, prefix, hash }) {
    this.keys.set(hash, { account_id, kind, prefix, hash });
  }

  getKeyByHash(hash) {
    return this.keys.get(hash) ?? null;
  }

  insertEmailToken({ hash, account_id, expires_at }) {
    this.tokens.set(hash, { hash, account_id, expires_at });
  }

  consumeEmailToken(hash) {
    const row = this.tokens.get(hash);
    if (!row) {
      return null;
    }
    this.tokens.delete(hash);
    return row;
  }

  usageKey(accountId, period) {
    return `${accountId}:${period}`;
  }

  getUsage(accountId, period) {
    return this.usage.get(this.usageKey(accountId, period)) ?? 0;
  }

  setUsage(accountId, period, n) {
    this.usage.set(this.usageKey(accountId, period), n);
  }

  addUsage(accountId, period, n) {
    const next = this.getUsage(accountId, period) + n;
    this.setUsage(accountId, period, next);
    return next;
  }

  hasStripeEvent(id) {
    return this.stripeEvents.has(id);
  }

  recordStripeEvent(id) {
    this.stripeEvents.add(id);
  }

  insertWebhook(row) {
    this.webhooks.set(row.id, { ...row });
    return this.webhooks.get(row.id);
  }

  listWebhooks(accountId) {
    return [...this.webhooks.values()].filter((row) => row.account_id === accountId);
  }

  getWebhook(id) {
    return this.webhooks.get(id) ?? null;
  }

  deleteWebhook(id, accountId) {
    const row = this.webhooks.get(id);
    if (!row || row.account_id !== accountId) {
      return false;
    }
    this.webhooks.delete(id);
    return true;
  }

  insertDelivery(row) {
    this.deliveries.set(row.id, { ...row });
    return this.deliveries.get(row.id);
  }

  getDelivery(id) {
    return this.deliveries.get(id) ?? null;
  }

  listDeliveries(accountId) {
    const ids = new Set(this.listWebhooks(accountId).map((row) => row.id));
    return [...this.deliveries.values()]
      .filter((row) => ids.has(row.endpoint_id))
      .sort((a, b) => (a.created_at < b.created_at ? 1 : -1));
  }

  updateDelivery(id, patch) {
    const row = this.deliveries.get(id);
    if (!row) {
      return null;
    }
    Object.assign(row, patch);
    return row;
  }

  dump() {
    return {
      api_keys: [...this.keys.values()].map((row) => ({
        hash: row.hash,
        kind: row.kind,
        prefix: row.prefix,
        account_id: row.account_id,
      })),
      accounts: [...this.accounts.values()],
    };
  }
}
