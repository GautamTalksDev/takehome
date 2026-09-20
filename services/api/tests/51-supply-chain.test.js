/**
 * A03:2025 Software Supply Chain Failures — items 20–26.
 * File checks, not notes: a workflow that uses a mutable tag, omits job
 * permissions, runs `npm install`, or publishes with a long-lived token
 * fails here before it can run with repo secrets.
 */
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const WORKFLOWS_DIR = path.join(REPO, '.github/workflows');
const SHA = /^[0-9a-f]{40}$/;

function workflowFiles() {
  return readdirSync(WORKFLOWS_DIR)
    .filter((name) => name.endsWith('.yml') || name.endsWith('.yaml'))
    .map((name) => ({
      name,
      text: readFileSync(path.join(WORKFLOWS_DIR, name), 'utf8'),
    }));
}

function stripComment(line) {
  return line.replace(/\s+#.*$/, '');
}

function usesRefs(text) {
  const refs = [];
  for (const raw of text.split('\n')) {
    const line = stripComment(raw);
    const match = line.match(/^\s+(?:-\s+)?uses:\s+(\S+)\s*$/);
    if (!match) continue;
    refs.push(match[1]);
  }
  return refs;
}

function parseJobs(text) {
  const jobsMatch = text.match(/^jobs:\s*\n([\s\S]*)$/m);
  assert.ok(jobsMatch, 'workflow has a jobs: block');
  const block = jobsMatch[1];
  const hits = [...block.matchAll(/^  ([A-Za-z0-9_-]+):\s*$/gm)];
  assert.ok(hits.length > 0, 'workflow has at least one job');
  return hits.map((hit, i) => {
    const start = hit.index + hit[0].length;
    const end = i + 1 < hits.length ? hits[i + 1].index : block.length;
    return { name: hit[1], body: block.slice(start, end) };
  });
}

function allWorkflowText() {
  return workflowFiles()
    .map((file) => file.text)
    .join('\n');
}

test('20. every GitHub Action is pinned to a full commit SHA', () => {
  const files = workflowFiles();
  assert.ok(files.length >= 2, 'expected test and publish/rules-watch workflows');
  let uses = 0;
  for (const file of files) {
    for (const ref of usesRefs(file.text)) {
      if (ref.startsWith('./') || ref.startsWith('docker://')) continue;
      uses += 1;
      const at = ref.lastIndexOf('@');
      assert.ok(at > 0, `${file.name}: malformed uses: ${ref}`);
      const pin = ref.slice(at + 1);
      assert.match(
        pin,
        SHA,
        `${file.name}: ${ref} is not a 40-char SHA (tags are mutable)`,
      );
    }
  }
  assert.ok(uses >= 8, `expected several third-party actions, found ${uses}`);
});

test('21. every workflow job sets explicit minimal permissions', () => {
  for (const file of workflowFiles()) {
    for (const job of parseJobs(file.text)) {
      assert.match(
        job.body,
        /^\s+permissions:\s*$/m,
        `${file.name} job ${job.name} missing permissions:`,
      );
      assert.match(
        job.body,
        /^\s+contents:\s+(read|write)\s*$/m,
        `${file.name} job ${job.name} must set contents: read or write`,
      );
      assert.doesNotMatch(
        job.body,
        /permissions:\s*write-all/,
        `${file.name} job ${job.name} must not use write-all`,
      );
    }
  }
});

test('22. CI runs npm audit --audit-level=high and pip-audit', () => {
  const text = allWorkflowText();
  assert.match(text, /npm audit --audit-level=high/);
  assert.match(text, /pip-audit/);
  for (const dir of [
    'services/api',
    'web/site',
    'packages/takehome-js',
    'tools/pdoc-oracle',
    'tools/rules-watch',
  ]) {
    assert.ok(
      text.includes(dir),
      `CI must audit ${dir}`,
    );
  }
  assert.match(text, /packages\/takehome-py/);
  assert.match(text, /cargo-deny-action@[0-9a-f]{40}/);
});

test('23. lockfiles are committed and CI installs with npm ci', () => {
  const cargoLocks = [
    path.join(REPO, 'Cargo.lock'),
    path.join(REPO, 'packages/takehome-py/Cargo.lock'),
  ];
  for (const lock of cargoLocks) {
    assert.equal(existsSync(lock), true, lock);
  }

  const npmRoots = [
    'services/api',
    'web/site',
    'packages/takehome-js',
    'tools/pdoc-oracle',
    'tools/rules-watch',
  ];
  for (const dir of npmRoots) {
    const pkg = path.join(REPO, dir, 'package.json');
    const lock = path.join(REPO, dir, 'package-lock.json');
    assert.equal(existsSync(pkg), true, pkg);
    assert.equal(existsSync(lock), true, `${dir} is missing package-lock.json`);
  }

  for (const file of workflowFiles()) {
    assert.doesNotMatch(
      file.text,
      /\bnpm install\b/,
      `${file.name} must not run npm install`,
    );
    assert.doesNotMatch(
      file.text,
      /\bnpm i\b/,
      `${file.name} must not run npm i`,
    );
  }
  assert.match(allWorkflowText(), /\bnpm ci\b/);
});

test('24. Dependabot runs weekly for cargo, npm, pip, and github-actions', () => {
  const dependabotPath = path.join(REPO, '.github/dependabot.yml');
  assert.equal(existsSync(dependabotPath), true, 'missing .github/dependabot.yml');
  const text = readFileSync(dependabotPath, 'utf8');
  assert.match(text, /package-ecosystem:\s*cargo/);
  assert.match(text, /package-ecosystem:\s*npm/);
  assert.match(text, /package-ecosystem:\s*pip/);
  assert.match(text, /package-ecosystem:\s*github-actions/);
  const weekly = [...text.matchAll(/interval:\s*weekly/g)];
  assert.ok(weekly.length >= 4, `expected weekly cadence, found ${weekly.length}`);
  for (const dir of [
    '/services/api',
    '/web/site',
    '/packages/takehome-js',
    '/tools/pdoc-oracle',
    '/tools/rules-watch',
    '/packages/takehome-py',
  ]) {
    assert.ok(text.includes(dir), `Dependabot must cover ${dir}`);
  }
});

test('25. packages publish from GitHub Actions via OIDC, not long-lived tokens', () => {
  const publish = workflowFiles().find((file) => file.name === 'publish.yml');
  assert.ok(publish, 'missing .github/workflows/publish.yml');
  assert.match(publish.text, /id-token:\s*write/);
  assert.match(publish.text, /npm publish --provenance/);
  assert.match(publish.text, /pypa\/gh-action-pypi-publish@[0-9a-f]{40}/);
  assert.doesNotMatch(
    publish.text,
    /\$\{\{\s*secrets\.(NPM_TOKEN|PYPI_API_TOKEN|NODE_AUTH_TOKEN)/,
  );
  assert.doesNotMatch(publish.text, /^\s+NODE_AUTH_TOKEN:/m);
  assert.doesNotMatch(publish.text, /MATURIN_PYPI_TOKEN/);

  const script = readFileSync(
    path.join(REPO, 'scripts/publish-packages.sh'),
    'utf8',
  );
  assert.match(script, /provenance|Trusted Publishing/i);
  assert.doesNotMatch(script, /export NODE_AUTH_TOKEN/);
  assert.doesNotMatch(script, /maturin publish --username/);
});

test('26. the release workflow generates an SBOM and attaches it', () => {
  const publish = workflowFiles().find((file) => file.name === 'publish.yml');
  assert.ok(publish, 'missing .github/workflows/publish.yml');
  assert.match(publish.text, /cargo sbom|cargo-sbom/);
  assert.match(publish.text, /cyclonedx-npm/);
  assert.match(publish.text, /gh release upload/);
});
