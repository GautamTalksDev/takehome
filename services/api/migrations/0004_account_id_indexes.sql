-- Production D1 (3cfd409e) recorded 0001/0002 as applied but drifted from the
-- migration files: named account_id indexes were absent, and
-- webhook_deliveries lacked account_id (BOLA scoping column). Staging is
-- correct. Empty deliveries table on prod makes ADD COLUMN safe.
ALTER TABLE webhook_deliveries ADD COLUMN account_id TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_api_keys_account_id ON api_keys(account_id);
CREATE INDEX IF NOT EXISTS idx_email_tokens_account_id ON email_tokens(account_id);
CREATE INDEX IF NOT EXISTS idx_usage_account_id ON usage(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_account_id ON webhook_endpoints(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_account_id ON webhook_deliveries(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint_id ON webhook_deliveries(endpoint_id);
