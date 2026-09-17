import { copyFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '../../..');
const dest = path.join(here, '..', 'src', 'assets');
mkdirSync(dest, { recursive: true });
copyFileSync(
  path.join(repo, 'conformance.json'),
  path.join(dest, 'conformance.json'),
);
copyFileSync(
  path.join(repo, 'web', 'site', 'src', 'data', 'changelog.json'),
  path.join(dest, 'changelog.json'),
);
copyFileSync(
  path.join(repo, 'data', 'plans.json'),
  path.join(dest, 'plans.json'),
);
