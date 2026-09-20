-- Repair drift: named account_id indexes may be missing even when 0001/0002
-- are marked applied. Fresh databases already get these from 0001/0002;
-- IF NOT EXISTS keeps this migration a no-op there. webhook_deliveries
-- account_id is defined in 0002; production drift that lacked the column
-- was repaired once when this migration first ran (ADD COLUMN). Do not
-- re-ADD here — that breaks fresh applies where 0002 already created it.
CREATE INDEX IF NOT EXISTS idx_api_keys_account_id ON api_keys(account_id);
CREATE INDEX IF NOT EXISTS idx_email_tokens_account_id ON email_tokens(account_id);
CREATE INDEX IF NOT EXISTS idx_usage_account_id ON usage(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_account_id ON webhook_endpoints(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_account_id ON webhook_deliveries(account_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint_id ON webhook_deliveries(endpoint_id);
