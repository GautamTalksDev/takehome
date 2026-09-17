export class MemoryStore {
  constructor() {
    this.accounts = new Map();
    this.keys = new Map();
    this.tokens = new Map();
    this.usage = new Map();
    this.stripeEvents = new Set();
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
