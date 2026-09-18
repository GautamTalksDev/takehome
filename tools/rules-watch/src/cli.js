#!/usr/bin/env node
/**
 * Scheduled T4127 pin check (spec §12.3). Exit 1 if fetch fails; exit 0
 * after opening issues (or when every pin still matches).
 */

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { watch } from './watch.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../..');
const pins = JSON.parse(
  readFileSync(path.join(here, 'pins.json'), 'utf8'),
);

async function openIssue({ title, body }) {
  const repo = process.env.GITHUB_REPOSITORY;
  const token = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN;
  if (!repo || !token) {
    console.error('GITHUB_REPOSITORY and GITHUB_TOKEN are required to open an issue');
    console.error(`# ${title}\n${body}`);
    throw new Error('missing GitHub credentials');
  }
  const response = await fetch(`https://api.github.com/repos/${repo}/issues`, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${token}`,
      accept: 'application/vnd.github+json',
      'x-github-api-version': '2022-11-28',
    },
    body: JSON.stringify({
      title,
      body,
      labels: ['t4127', 'rules-watch'],
    }),
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`GitHub issue create ${response.status}: ${text}`);
  }
}

function readSnapshot(rel) {
  return readFileSync(path.join(repo, rel), 'utf8');
}

const result = await watch({
  pins,
  fetchImpl: globalThis.fetch.bind(globalThis),
  openIssue,
  readSnapshot,
});

if (result.issues.length === 0) {
  console.log('T4127 pins unchanged');
} else {
  console.log(`opened ${result.issues.length} issue(s)`);
  for (const issue of result.issues) {
    console.log(`- ${issue.title}`);
  }
}
