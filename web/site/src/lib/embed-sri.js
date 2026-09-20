import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

function siteRoot() {
  let dir = path.dirname(fileURLToPath(import.meta.url));
  for (let i = 0; i < 10; i += 1) {
    if (existsSync(path.join(dir, 'public', 'embed.js'))) {
      return dir;
    }
    dir = path.dirname(dir);
  }
  if (existsSync(path.join(process.cwd(), 'public', 'embed.js'))) {
    return process.cwd();
  }
  throw new Error('public/embed.js not found from embed-sri.js');
}

export const EMBED_VERSION = 'v0.1.0';
export const EMBED_PATH = `/embed/${EMBED_VERSION}/embed.js`;
export const EMBED_URL = `https://takehome.gautamkhosla.com${EMBED_PATH}`;

export function embedSri() {
  const src = readFileSync(path.join(siteRoot(), 'public/embed.js'));
  return `sha384-${createHash('sha384').update(src).digest('base64')}`;
}

export function embedSnippet() {
  return `<script src="${EMBED_URL}" integrity="${embedSri()}" crossorigin="anonymous" data-province="ON"
        data-gross="75000"></script>`;
}
