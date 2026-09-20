CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  email_verified INTEGER NOT NULL DEFAULT 0,
  plan TEXT NOT NULL DEFAULT 'developer',
  stripe_customer_id TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE api_keys (
  hash TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  prefix TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);

CREATE INDEX idx_api_keys_account_id ON api_keys(account_id);

CREATE TABLE email_tokens (
  hash TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);

CREATE INDEX idx_email_tokens_account_id ON email_tokens(account_id);

CREATE TABLE usage (
  account_id TEXT NOT NULL,
  period_start TEXT NOT NULL,
  calculations INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (account_id, period_start),
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);

CREATE INDEX idx_usage_account_id ON usage(account_id);

CREATE TABLE stripe_events (
  id TEXT PRIMARY KEY
);
