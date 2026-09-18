/**
 * Pin T4127 remote documents, fetch, canonicalize, hash, open an issue
 * with the unified diff on drift (spec §12.3).
 */

import { createHash } from 'node:crypto';
import { canonicalize, csvHrefs } from './canonicalize.js';
import { unifiedDiff } from './diff.js';

export function sha256(text) {
  return createHash('sha256').update(text, 'utf8').digest('hex');
}

export async function watch({
  pins,
  fetchImpl,
  openIssue,
  readSnapshot,
}) {
  const issues = [];
  const seenCsv = new Set();

  for (const doc of pins.documents ?? []) {
    const fetched = await getCanonical(fetchImpl, doc.url, doc.kind);
    if (doc.kind === 'html') {
      const hrefs = csvHrefs(fetched.canonical, doc.url);
      for (const href of hrefs) {
        const name = href.split('/').pop().split('?')[0];
        const recorded = (pins.csv_bundle ?? []).find((row) => row.name === name);
        if (!recorded) {
          continue;
        }
        if (seenCsv.has(name)) {
          continue;
        }
        seenCsv.add(name);
        const csv = await getCanonical(fetchImpl, href, 'csv');
        const drift = compare({
          id: `csv:${name}`,
          url: href,
          kind: 'csv',
          sha256: recorded.sha256,
          snapshot: recorded.snapshot,
          fetched: csv,
          readSnapshot,
        });
        if (drift) {
          await openIssue(drift);
          issues.push(drift);
        }
      }
    }
    if (!doc.sha256) {
      continue;
    }
    const drift = compare({
      id: doc.id,
      url: doc.url,
      kind: doc.kind,
      sha256: doc.sha256,
      snapshot: doc.snapshot,
      fetched,
      readSnapshot,
    });
    if (drift) {
      await openIssue(drift);
      issues.push(drift);
    }
  }

  for (const recorded of pins.csv_bundle ?? []) {
    if (!recorded.url || seenCsv.has(recorded.name)) {
      continue;
    }
    seenCsv.add(recorded.name);
    const csv = await getCanonical(fetchImpl, recorded.url, 'csv');
    const drift = compare({
      id: `csv:${recorded.name}`,
      url: recorded.url,
      kind: 'csv',
      sha256: recorded.sha256,
      snapshot: recorded.snapshot,
      fetched: csv,
      readSnapshot,
    });
    if (drift) {
      await openIssue(drift);
      issues.push(drift);
    }
  }

  return { issues };
}

async function getCanonical(fetchImpl, url, kind) {
  const response = await fetchImpl(url);
  if (!response.ok) {
    throw new Error(`GET ${url} -> ${response.status}`);
  }
  const buf = Buffer.from(await response.arrayBuffer());
  const canonical = canonicalize(kind, buf);
  return { canonical, digest: sha256(canonical) };
}

function compare({ id, url, kind, sha256: recorded, snapshot, fetched, readSnapshot }) {
  if (fetched.digest === recorded) {
    return null;
  }
  const previousRaw = snapshot && readSnapshot ? readSnapshot(snapshot) : '';
  const previous = previousRaw ? canonicalize(kind, previousRaw) : '';
  const diff = unifiedDiff(previous, fetched.canonical, `pin/${id}`, `fetched/${id}`);
  return {
    title: `T4127 pin drift: ${id}`,
    body: [
      `Recorded sha256 \`${recorded}\`.`,
      `Fetched sha256 \`${fetched.digest}\`.`,
      `URL: ${url}`,
      '',
      '```diff',
      diff,
      '```',
      '',
    ].join('\n'),
    id,
    diff,
  };
}
