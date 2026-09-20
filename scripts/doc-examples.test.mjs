#!/usr/bin/env node
/**
 * Extract fenced blocks tagged `runnable` and execute them against a local API.
 * A documentation example that does not run is a bug.
 *
 * Expects TAKEHOME_DOC_API (default http://127.0.0.1:8787) and
 * TAKEHOME_DOC_API_KEY (np_test_… from signup when required).
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const API = process.env.TAKEHOME_DOC_API ?? 'http://127.0.0.1:8787';
const KEY = process.env.TAKEHOME_DOC_API_KEY ?? '';

function walkMd(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (name === 'node_modules' || name === 'target' || name === 'pdoc-cache') continue;
      walkMd(full, acc);
    } else if (name.endsWith('.md')) {
      acc.push(full);
    }
  }
  return acc;
}

function extractRunnable(source, file) {
  const out = [];
  const re = /```(bash|sh|javascript|js|python|py)[^\n]*\brunable\b[^\n]*\n([\s\S]*?)```/g;
  let m;
  while ((m = re.exec(source)) !== null) {
    out.push({
      file: path.relative(ROOT, file),
      lang: m[1],
      body: m[2].replace(/https:\/\/takehome\.gautamkhosla\.com/g, API),
    });
  }
  return out;
}

function collect() {
  const files = [
    ...walkMd(path.join(ROOT, 'docs')),
    path.join(ROOT, 'README.md'),
    path.join(ROOT, 'ARCHITECTURE.md'),
    path.join(ROOT, 'CONTRIBUTING.md'),
    path.join(ROOT, 'SECURITY.md'),
  ].filter((f) => {
    try {
      statSync(f);
      return true;
    } catch {
      return false;
    }
  });
  return files.flatMap((f) => extractRunnable(readFileSync(f, 'utf8'), f));
}

const examples = collect();

test('at least one runnable documentation example exists', () => {
  assert.ok(examples.length > 0, 'no ```… runnable fences found');
});

test('runnable examples execute against the local API', async (t) => {
  if (!KEY && examples.some((e) => /Bearer|authorization/i.test(e.body))) {
    t.skip('set TAKEHOME_DOC_API_KEY to run authenticated doc examples');
    return;
  }
  // Health check
  const health = await fetch(`${API}/v1/health`).catch(() => null);
  if (!health || !health.ok) {
    t.skip(`local API not reachable at ${API}; start services/api/scripts/local.mjs`);
    return;
  }

  for (const ex of examples) {
    await t.test(`${ex.file} (${ex.lang})`, async () => {
      let body = ex.body;
      if (KEY) {
        body = body.replace(/np_test_YOUR_KEY/g, KEY);
      }
      if (/np_test_YOUR_KEY/.test(body)) {
        assert.fail('example still contains np_test_YOUR_KEY; set TAKEHOME_DOC_API_KEY');
      }
      const dir = mkdtempSync(path.join(tmpdir(), 'doc-ex-'));
      if (ex.lang === 'bash' || ex.lang === 'sh') {
        const script = path.join(dir, 'run.sh');
        writeFileSync(script, body);
        const r = spawnSync('bash', [script], {
          encoding: 'utf8',
          env: { ...process.env, TAKEHOME_DOC_API: API, TAKEHOME_DOC_API_KEY: KEY },
        });
        assert.equal(r.status, 0, r.stderr || r.stdout);
      } else if (ex.lang === 'javascript' || ex.lang === 'js') {
        const script = path.join(dir, 'run.mjs');
        writeFileSync(script, body);
        const r = spawnSync(process.execPath, [script], {
          encoding: 'utf8',
          env: { ...process.env, TAKEHOME_DOC_API: API, TAKEHOME_DOC_API_KEY: KEY },
        });
        assert.equal(r.status, 0, r.stderr || r.stdout);
      } else if (ex.lang === 'python' || ex.lang === 'py') {
        const script = path.join(dir, 'run.py');
        writeFileSync(script, body);
        const r = spawnSync('python3', [script], {
          encoding: 'utf8',
          env: { ...process.env, TAKEHOME_DOC_API: API, TAKEHOME_DOC_API_KEY: KEY },
        });
        assert.equal(r.status, 0, r.stderr || r.stdout);
      }
    });
  }
});
