import { createServer } from 'node:http';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { MemoryStore } from '../src/store-memory.js';

const port = Number.parseInt(process.env.PORT ?? '8787', 10);
const site = process.env.PUBLIC_SITE ?? 'http://127.0.0.1:4321';
const store = new MemoryStore();
const mailbox = [];

const env = {
  CLOCK_DATE: process.env.CLOCK_DATE ?? '2026-09-16',
  RATE_LIMIT_MAX: '10000',
  RATE_LIMIT_WINDOW_MS: '60000',
  STORE: store,
  MAILBOX: mailbox,
  ECHO_VERIFY_URL: process.env.ECHO_VERIFY_URL ?? '1',
  PUBLIC_SITE: site,
  STRIPE_PRICE_STARTER: 'price_starter_cad',
  STRIPE_PRICE_GROWTH: 'price_growth_cad',
  STRIPE_PRICE_BUSINESS: 'price_business_cad',
};

const server = createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => {
    chunks.push(chunk);
  });
  req.on('end', () => {
    void dispatch(req, res, Buffer.concat(chunks));
  });
});

async function dispatch(req, res, body) {
  try {
    const headers = new Headers();
    for (const [name, value] of Object.entries(req.headers)) {
      if (value == null) {
        continue;
      }
      if (Array.isArray(value)) {
        for (const item of value) {
          headers.append(name, item);
        }
      } else {
        headers.set(name, value);
      }
    }
    const method = req.method ?? 'GET';
    const hasBody = method !== 'GET' && method !== 'HEAD';
    const request = new Request(`http://127.0.0.1:${port}${req.url}`, {
      method,
      headers,
      body: hasBody ? body : undefined,
    });
    const response = await handle(request, env, nodeEngine);
    const out = Buffer.from(await response.arrayBuffer());
    const outHeaders = {};
    response.headers.forEach((value, name) => {
      outHeaders[name] = value;
    });
    res.writeHead(response.status, outHeaders);
    res.end(out);
  } catch (err) {
    res.writeHead(500, { 'content-type': 'application/json' });
    res.end(
      JSON.stringify({ error: { code: 'engine', message: String(err) } }),
    );
  }
}

server.listen(port, '127.0.0.1', () => {
  process.stdout.write(
    `takehome-api local ${port}; PUBLIC_SITE=${site}; ECHO_VERIFY_URL=${env.ECHO_VERIFY_URL}\n`,
  );
});
