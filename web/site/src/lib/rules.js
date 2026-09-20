import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

function findRepoRoot() {
  const starts = [process.cwd(), path.dirname(fileURLToPath(import.meta.url))];
  for (const start of starts) {
    let dir = start;
    for (let i = 0; i < 10; i += 1) {
      if (existsSync(path.join(dir, 'data', 'rules', '2026-01-01', 'manifest.json'))) {
        return dir;
      }
      const parent = path.dirname(dir);
      if (parent === dir) {
        break;
      }
      dir = parent;
    }
  }
  throw new Error('cannot find repo root (data/rules/2026-01-01/manifest.json)');
}

export const REPO = findRepoRoot();

const FILE_BY_SLUG = {
  ab: 'ab.json',
  bc: 'bc.json',
  mb: 'mb.json',
  nb: 'nb.json',
  nl: 'nl.json',
  ns: 'ns.json',
  nt: 'nt.json',
  nu: 'nu.json',
  on: 'on.json',
  pe: 'pe.json',
  sk: 'sk.json',
  yt: 'yt.json',
};

export function rulesDir(version) {
  return path.join(REPO, 'data', 'rules', version);
}

export function readJson(version, file) {
  return JSON.parse(readFileSync(path.join(rulesDir(version), file), 'utf8'));
}

export function loadEdition(version) {
  return {
    manifest: readJson(version, 'manifest.json'),
    federal: readJson(version, 'federal.json'),
    cpp: readJson(version, 'cpp.json'),
    ei: readJson(version, 'ei.json'),
  };
}

export function loadProvincial(version, slug) {
  const file = FILE_BY_SLUG[slug];
  if (!file) {
    return null;
  }
  return readJson(version, file);
}

export function bracketRows(brackets) {
  if (!brackets) {
    return null;
  }
  if (Array.isArray(brackets)) {
    return { option1: brackets };
  }
  return brackets;
}

export function bpaText(bpa) {
  if (!bpa) {
    return 'None in this file.';
  }
  if (bpa.option1 || bpa.option2) {
    const parts = [];
    if (bpa.option1) {
      parts.push(`Option 1: ${describeBpa(bpa.option1)}`);
    }
    if (bpa.option2) {
      parts.push(`Option 2: ${describeBpa(bpa.option2)}`);
    }
    return parts.join(' ');
  }
  return describeBpa(bpa);
}

function describeBpa(bpa) {
  if (bpa.type === 'same_as_federal') {
    return 'same as federal BPAF (including the phase-out).';
  }
  if (bpa.type === 'dynamic') {
    return `dynamic, max ${money(bpa.max)}, min ${money(bpa.min)}, phase-out ${money(bpa.phaseout_start)} to ${money(bpa.phaseout_end)}.`;
  }
  if (bpa.amount) {
    return `fixed ${money(bpa.amount)}.`;
  }
  return JSON.stringify(bpa);
}

export function money(value) {
  if (value == null) {
    return '—';
  }
  const n = String(value);
  if (n.includes('.')) {
    return `$${n}`;
  }
  return `$${n}.00`;
}

export function rate(value) {
  if (value == null) {
    return '—';
  }
  if (typeof value === 'object') {
    return ['option1', 'option2']
      .filter((key) => value[key] != null)
      .map((key) => `${key === 'option1' ? 'Option 1' : 'Option 2'} ${rate(value[key])}`)
      .join(', ');
  }
  const token = String(value);
  if (!token.startsWith('0.')) {
    return token;
  }
  const frac = token.slice(2);
  const padded = frac.padEnd(4, '0');
  const whole = padded.slice(0, padded.length - 2).replace(/^0+/, '') || '0';
  const places = padded.slice(padded.length - 2);
  return `${whole}.${places}%`;
}
