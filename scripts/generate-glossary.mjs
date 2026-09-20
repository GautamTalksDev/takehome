#!/usr/bin/env node
/**
 * Regenerate docs/glossary.md from data/factors.json.
 * Run: node scripts/generate-glossary.mjs
 */
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const FACTORS_PATH = path.join(ROOT, 'data/factors.json');
const OUT_PATH = path.join(ROOT, 'docs/glossary.md');

const CRA_URL =
  'https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/payroll/payroll-deductions-formulas.html';

const { factors } = JSON.parse(readFileSync(FACTORS_PATH, 'utf8'));

const lines = [
  '# Glossary of factor symbols',
  '',
  '**Who this is for:** Developers reading API `breakdown` fields or CONFORMANCE vectors who need plain English for each symbol.',
  '',
  '**When you finish:** You can map any factor symbol in a response to its T4127 (Payroll Deductions Formulas) step and open the CRA payroll formulas hub.',
  '',
  'Amounts in API responses are decimal strings in cents; this page defines names only.',
  '',
];

for (const factor of factors) {
  const { symbol, definition, t4127_reference } = factor;
  lines.push(`## ${symbol}`);
  lines.push('');
  lines.push(definition.endsWith('.') ? definition : `${definition}.`);
  lines.push('');
  lines.push(`**Source:** ${t4127_reference}`);
  lines.push('');
  lines.push(`**CRA:** ${CRA_URL}`);
  lines.push('');
}

lines.push('**Last reviewed:** 2026-09-20');
lines.push('**Engine:** 0.1.0');
lines.push('');

writeFileSync(OUT_PATH, lines.join('\n'), 'utf8');
console.log(`Wrote ${path.relative(ROOT, OUT_PATH)} (${factors.length} factors)`);
