import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { MemoryStore } from '../src/store-memory.js';
import { issueKeyPair } from '../src/keys.js';

export const BASE_ENV = {
  CLOCK_DATE: '2026-09-16',
  RATE_LIMIT_MAX: '10000',
  RATE_LIMIT_WINDOW_MS: '60000',
  STRIPE_PRICE_STARTER: 'price_starter_cad',
  STRIPE_PRICE_GROWTH: 'price_growth_cad',
  STRIPE_PRICE_BUSINESS: 'price_business_cad',
  PUBLIC_SITE: 'https://takehome.gautamkhosla.com',
};

export function createWorld(plan = 'developer') {
  const store = new MemoryStore();
  const mailbox = [];
  const stripeSessions = [];
  const account = store.createAccount({
    email: `dev-${store.accounts.size}@example.com`,
    email_verified: true,
    plan,
  });
  const keys = issueKeyPair(store, account.id);
  const env = {
    ...BASE_ENV,
    STORE: store,
    MAILBOX: mailbox,
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
  return { env, store, account, mailbox, stripeSessions, ...keys };
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
