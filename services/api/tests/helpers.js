import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { resetSecurityLog } from '../src/log.js';
import { issueKeyPair } from '../src/keys.js';
import { MemoryStore } from '../src/store-memory.js';
import { SqliteStore } from './store-sqlite.js';

export function newStore() {
  if (process.env.TAKEHOME_API_STORE === 'd1') {
    return SqliteStore.fromMigrations();
  }
  return new MemoryStore();
}

export const BASE_ENV = {
  CLOCK_DATE: '2026-09-16',
  RATE_LIMIT_MAX: '10000',
  RATE_LIMIT_WINDOW_MS: '60000',
  SIGNUP_RATE_LIMIT_MAX: '10000',
  VERIFY_RATE_LIMIT_MAX: '10000',
  STRIPE_PRICE_STARTER: 'price_starter_cad',
  STRIPE_PRICE_GROWTH: 'price_growth_cad',
  STRIPE_PRICE_BUSINESS: 'price_business_cad',
  PUBLIC_SITE: 'https://takehome.gautamkhosla.com',
};

export function createWorld(plan = 'developer') {
  resetSecurityLog();
  const store = newStore();
  const mailbox = [];
  const alerts = [];
  const stripeSessions = [];
  const account = store.createAccount({
    email: `dev-${store.accounts.size}@example.com`,
    email_verified: true,
    plan,
  });
  const keys = issueKeyPair(store, account.id);
  const env = {
    ...BASE_ENV,
    WEBHOOK_RESOLVE: async () => ['203.0.113.10'],
    STORE: store,
    MAILBOX: mailbox,
    ALERTS: alerts,
    STRIPE: {
      createCheckoutSession(input) {
        const session = {
          id: `cs_test_${stripeSessions.length + 1}`,
          url: `https://checkout.stripe.com/c/pay/cs_test_${stripeSessions.length + 1}`,
          currency: 'cad',
          ...input,
        };
        stripeSessions.push(session);
        return session;
      },
      constructEvent(raw) {
        return JSON.parse(raw);
      },
    },
  };
  return { env, store, account, mailbox, alerts, stripeSessions, ...keys };
}

export function addAccount(world, plan = 'developer') {
  const account = world.store.createAccount({
    email: `dev-${world.store.accounts.size}@example.com`,
    email_verified: true,
    plan,
  });
  const keys = issueKeyPair(world.store, account.id);
  return { account, ...keys };
}

const defaultWorld = createWorld();

export const ENV = defaultWorld.env;
export const fixtures = defaultWorld;

export function request(method, path, body, headers = {}) {
  const init = {
    method,
    headers: { ...headers },
  };
  if (body !== undefined) {
    init.headers['content-type'] =
      init.headers['content-type'] ?? 'application/json';
    init.body = typeof body === 'string' ? body : JSON.stringify(body);
  }
  return new Request(`https://takehome.gautamkhosla.com${path}`, init);
}

function hasAuthorization(headers) {
  return Object.keys(headers).some((key) => key.toLowerCase() === 'authorization');
}

export async function call(method, path, body, headers = {}, world = defaultWorld) {
  const extra = { ...headers };
  const omitAuth = extra.authorization === null;
  if (omitAuth) {
    delete extra.authorization;
  } else if (
    method === 'POST' &&
    path.startsWith('/v1/deductions') &&
    !hasAuthorization(extra)
  ) {
    extra.authorization = `Bearer ${world.testKey}`;
  }
  const response = await handle(
    request(method, path, body, extra),
    world.env,
    nodeEngine,
  );
  const text = await response.text();
  let json = null;
  try {
    json = JSON.parse(text);
  } catch {
    json = null;
  }
  return { response, text, json, status: response.status };
}

export const ON_WEEKLY = {
  as_of: '2026-01-15',
  province: 'ON',
  pay_period: 52,
  gross_pay: '1000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
};
