export class MemoryStore {
  constructor() {
    this.accounts = new Map();
    this.keys = new Map();
    this.tokens = new Map();
    this.usage = new Map();
    this.stripeEvents = new Set();
    this.webhooks = new Map();
    this.webhooksByAccount = new Map();
    this.deliveries = new Map();
    this.deliveriesByAccount = new Map();
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

  insertKey({ account_id, kind, prefix, hash, revoked = false }) {
    this.keys.set(hash, {
      account_id,
      kind,
      prefix,
      hash,
      revoked: Boolean(revoked),
    });
  }

  getKeyByHash(hash) {
    return this.keys.get(hash) ?? null;
  }

  listKeys(accountId) {
    return [...this.keys.values()].filter((row) => row.account_id === accountId);
  }

  revokeKind(accountId, kind) {
    for (const row of this.keys.values()) {
      if (row.account_id === accountId && row.kind === kind) {
        row.revoked = true;
      }
    }
  }

  insertEmailToken({ hash, account_id, expires_at }) {
    this.tokens.set(hash, { hash, account_id, expires_at });
  }

  getEmailToken(hash) {
    return this.tokens.get(hash) ?? null;
  }

  deleteEmailToken(hash) {
    this.tokens.delete(hash);
  }

  deleteEmailTokensForAccount(accountId) {
    for (const [hash, row] of this.tokens) {
      if (row.account_id === accountId) {
        this.tokens.delete(hash);
      }
    }
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
    accountIndex(this.webhooksByAccount, row.account_id).add(row.id);
    return this.webhooks.get(row.id);
  }

  listWebhooks(accountId) {
    const ids = this.webhooksByAccount.get(accountId);
    if (!ids) {
      return [];
    }
    return [...ids].map((id) => this.webhooks.get(id)).filter(Boolean);
  }

  getWebhook(id, accountId) {
    const row = this.webhooks.get(id);
    if (!row || row.account_id !== accountId) {
      return null;
    }
    return row;
  }

  deleteWebhook(id, accountId) {
    const row = this.webhooks.get(id);
    if (!row || row.account_id !== accountId) {
      return false;
    }
    this.webhooks.delete(id);
    this.webhooksByAccount.get(accountId)?.delete(id);
    for (const [deliveryId, delivery] of this.deliveries) {
      if (delivery.endpoint_id === id) {
        this.deliveries.delete(deliveryId);
        this.deliveriesByAccount.get(delivery.account_id)?.delete(deliveryId);
      }
    }
    return true;
  }

  insertDelivery(row) {
    this.deliveries.set(row.id, { ...row });
    accountIndex(this.deliveriesByAccount, row.account_id).add(row.id);
    return this.deliveries.get(row.id);
  }

  getDelivery(id, accountId) {
    const row = this.deliveries.get(id);
    if (!row || row.account_id !== accountId) {
      return null;
    }
    return row;
  }

  listDeliveries(accountId) {
    const ids = this.deliveriesByAccount.get(accountId);
    if (!ids) {
      return [];
    }
    return [...ids]
      .map((id) => this.deliveries.get(id))
      .filter(Boolean)
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
        revoked: Boolean(row.revoked),
      })),
      accounts: [...this.accounts.values()],
    };
  }
}

function accountIndex(map, accountId) {
  let ids = map.get(accountId);
  if (!ids) {
    ids = new Set();
    map.set(accountId, ids);
  }
  return ids;
}
