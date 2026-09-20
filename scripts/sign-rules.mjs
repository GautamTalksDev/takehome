#!/usr/bin/env node
/**
 * Sign embedded T4127 rule JSON with Ed25519 (RFC 8032).
 * Message is path || 0x00 || file bytes. Seed: data/rules/signing.seed.
 */
import { createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const RULES = path.join(ROOT, 'data/rules');
const SEED_PATH = path.join(RULES, 'signing.seed');

const FILES = [
  ...edition('2026-01-01', [
    'manifest.json',
    'federal.json',
    'ab.json',
    'bc.json',
    'mb.json',
    'nb.json',
    'nl.json',
    'ns.json',
    'nt.json',
    'nu.json',
    'on.json',
    'pe.json',
    'sk.json',
    'yt.json',
    'cpp.json',
    'ei.json',
    'qpip.json',
  ]),
  ...edition('2026-07-01', [
    'manifest.json',
    'federal.json',
    'ab.json',
    'bc.json',
    'mb.json',
    'nb.json',
    'nl.json',
    'ns.json',
    'nt.json',
    'nu.json',
    'on.json',
    'pe.json',
    'sk.json',
    'yt.json',
    'cpp.json',
    'ei.json',
    'qpip.json',
  ]),
  ...edition('2027-01-01', [
    'manifest.json',
    'bc.json',
    'nl.json',
    'pe.json',
    'cpp.json',
  ]),
];

function edition(version, names) {
  return names.map((name) => `${version}/${name}`);
}

function loadSeed() {
  if (process.env.RULES_SIGNING_SEED) {
    return Buffer.from(process.env.RULES_SIGNING_SEED, 'hex');
  }
  return Buffer.from(readFileSync(SEED_PATH, 'utf8').trim(), 'hex');
}

function privateKeyFromSeed(seed) {
  if (seed.length !== 32) {
    throw new Error(`Ed25519 seed must be 32 bytes, got ${seed.length}`);
  }
  const pkcs8 = Buffer.concat([
    Buffer.from('302e020100300506032b657004220420', 'hex'),
    seed,
  ]);
  return createPrivateKey({ key: pkcs8, format: 'der', type: 'pkcs8' });
}

function publicHex(key) {
  const spki = createPublicKey(key).export({ type: 'spki', format: 'der' });
  return spki.subarray(-32).toString('hex');
}

function signedMessage(rel, bytes) {
  return Buffer.concat([Buffer.from(rel, 'utf8'), Buffer.from([0]), bytes]);
}

const seed = loadSeed();
const key = privateKeyFromSeed(seed);
const public_key = publicHex(key);
const files = {};
for (const rel of FILES) {
  const bytes = readFileSync(path.join(RULES, rel));
  const sig = sign(null, signedMessage(rel, bytes), key);
  files[rel] = sig.toString('hex');
}

const out = {
  algorithm: 'ed25519',
  public_key,
  files,
};
mkdirSync(RULES, { recursive: true });
writeFileSync(path.join(RULES, 'signatures.json'), `${JSON.stringify(out, null, 2)}\n`);
process.stdout.write(`signed ${FILES.length} files, public_key=${public_key}\n`);
