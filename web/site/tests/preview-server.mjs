/**
 * Serve Astro dist/ plus Playwright-only embed fixtures that must not ship.
 */
import { createServer } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');
const FIXTURES = path.join(ROOT, 'tests/fixtures');
const PORT = Number.parseInt(process.env.PLAYWRIGHT_PORT ?? '4321', 10);

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.xml': 'application/xml',
  '.txt': 'text/plain; charset=utf-8',
  '.map': 'application/json',
};

function contentType(file) {
  return TYPES[path.extname(file).toLowerCase()] ?? 'application/octet-stream';
}

function resolve(urlPath) {
  const clean = decodeURIComponent(urlPath.split('?')[0].split('#')[0]);
  const base = path.basename(clean);
  if (/^embed-.+\.html$/.test(base)) {
    const fixture = path.join(FIXTURES, base);
    if (existsSync(fixture)) {
      return fixture;
    }
  }
  const trimmed = clean.replace(/\/$/, '') || '/';
  if (trimmed === '/') {
    return path.join(DIST, 'index.html');
  }
  const nested = path.join(DIST, trimmed.slice(1), 'index.html');
  if (existsSync(nested)) {
    return nested;
  }
  const exact = path.join(DIST, trimmed.slice(1));
  if (existsSync(exact) && statSync(exact).isFile()) {
    return exact;
  }
  return null;
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? '/', `http://127.0.0.1:${PORT}`);
  const file = resolve(url.pathname);
  if (!file) {
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    res.end('not found');
    return;
  }
  const body = readFileSync(file);
  res.writeHead(200, { 'content-type': contentType(file) });
  res.end(body);
});

server.listen(PORT, '127.0.0.1', () => {
  process.stdout.write(`playwright-preview ${PORT}\n`);
});
