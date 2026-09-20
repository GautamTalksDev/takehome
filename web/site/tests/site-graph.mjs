/**
 * Spec §13.2 / test 10: catalog size, dist files, sitemap completeness,
 * internal link graph (no 404s).
 */
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { catalog, htmlPaths } from '../src/lib/catalog.js';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');
const SITE = 'https://takehome.gautamkhosla.com';

function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) {
      walk(full, acc);
    } else {
      acc.push(full);
    }
  }
  return acc;
}

function distFileFor(urlPath) {
  const trimmed = urlPath.replace(/\/$/, '');
  if (trimmed === '') {
    return path.join(DIST, 'index.html');
  }
  const nested = path.join(DIST, trimmed.slice(1), 'index.html');
  if (existsSync(nested)) {
    return nested;
  }
  const exact = path.join(DIST, trimmed.slice(1));
  return exact;
}

function extractHrefs(html) {
  const hrefs = [];
  const re = /href\s*=\s*["']([^"']+)["']/gi;
  let match;
  while ((match = re.exec(html))) {
    hrefs.push(match[1]);
  }
  return hrefs;
}

function internalPath(href) {
  if (!href || href.startsWith('#') || href.startsWith('mailto:')) {
    return null;
  }
  if (href.startsWith('http://') || href.startsWith('https://')) {
    if (!href.startsWith(SITE)) {
      return null;
    }
    href = href.slice(SITE.length) || '/';
  }
  if (!href.startsWith('/')) {
    return null;
  }
  const noHash = href.split('#')[0].split('?')[0];
  return noHash;
}

assert.ok(existsSync(DIST), 'dist missing; run npm run build first');

const pages = catalog();
assert.equal(pages.length, 205, `catalog must be 205 HTML pages, got ${pages.length}`);

const paths = htmlPaths();
const unique = new Set(paths);
assert.equal(unique.size, 205, 'catalog paths must be unique');

for (const urlPath of paths) {
  const file = distFileFor(urlPath);
  assert.ok(existsSync(file), `missing dist file for ${urlPath} (${file})`);
}

const sitemapPath = path.join(DIST, 'sitemap.xml');
assert.ok(existsSync(sitemapPath), 'sitemap.xml missing from dist');
const sitemap = readFileSync(sitemapPath, 'utf8');
for (const urlPath of paths) {
  const loc = `${SITE}${urlPath}`;
  assert.ok(sitemap.includes(`<loc>${loc}</loc>`), `sitemap missing ${loc}`);
}

const rssPath = path.join(DIST, 'changes.xml');
assert.ok(existsSync(rssPath), 'changes.xml RSS missing from dist');

const htmlFiles = walk(DIST).filter((file) => file.endsWith('.html'));
const missing = [];
for (const file of htmlFiles) {
  const html = readFileSync(file, 'utf8');
  for (const href of extractHrefs(html)) {
    const target = internalPath(href);
    if (!target) {
      continue;
    }
    const dest = distFileFor(target);
    if (!existsSync(dest)) {
      missing.push(`${path.relative(DIST, file)} -> ${target}`);
    }
  }
}
assert.equal(
  missing.length,
  0,
  `internal 404s:\n${missing.slice(0, 40).join('\n')}${missing.length > 40 ? `\n… ${missing.length} total` : ''}`,
);

const conformance = readFileSync(distFileFor('/conformance/'), 'utf8');
assert.match(conformance, /AB-P10-11704\.49-F0-P0/);
assert.match(conformance, /YT-P24-208\.33-F0-P0/);
const disagreementIds =
    conformance.match(/[A-Za-z]+-P\d+-[0-9.]+-F[01]-P[01]/g) || [];
const uniqueIds = new Set(disagreementIds);
assert.ok(
  uniqueIds.size >= 164,
  `conformance page must list all 164 disagreements, found ${uniqueIds.size}`,
);

const cpp2027 = readFileSync(distFileFor('/cpp-2027-rate-change/'), 'utf8');
assert.match(cpp2027, /2027/);
assert.match(cpp2027, /YMPE/);
assert.match(cpp2027, /YAMPE/);

const sales = [];
const banned = /contact sales|talk to sales|book a demo|request a demo/i;
for (const file of htmlFiles) {
  const html = readFileSync(file, 'utf8');
  if (banned.test(html)) {
    sales.push(path.relative(DIST, file));
  }
}
assert.equal(
  sales.length,
  0,
  `contact-sales copy is the wedge against Symmetry/CPTL:\n${sales.join('\n')}`,
);

const pricing = readFileSync(distFileFor('/pricing/'), 'utf8');
assert.match(pricing, /\$49 CAD/);
assert.match(pricing, /\$149 CAD/);
assert.match(pricing, /\$399 CAD/);
assert.match(pricing, /\$0 CAD/);

const docs = readFileSync(distFileFor('/docs/'), 'utf8');
assert.match(docs, /800\.79/);
assert.match(docs, /authorization: Bearer/i);
assert.match(docs, /\/signup\//);

const home = readFileSync(distFileFor('/'), 'utf8');
assert.match(home, /href="\/docs\/"/);
assert.match(home, /Get API keys/);
assert.match(home, /<svg[\s\S]*Takehome/);

const pricingHtml = pricing;
assert.match(pricingHtml, /class="plan"/);
assert.equal((pricingHtml.match(/class="plan"/g) || []).length, 5);

const docsHtml = docs;
assert.match(docsHtml, /aria-label="Docs"/);
assert.match(docsHtml, /class="shiki/);
assert.match(docsHtml, /data-copy/);

const css = readFileSync(path.join(ROOT, 'src/styles/site.css'), 'utf8');
const withoutTokens = css.replace(/:root\s*\{[^{}]*\}/g, '');
assert.equal(
  withoutTokens.match(/#[0-9a-fA-F]{3,8}\b/g),
  null,
  'literal hex belongs on :root tokens only',
);

assert.ok(existsSync(path.join(DIST, 'embed.js')), 'embed.js missing from dist');
const embedSrc = readFileSync(path.join(DIST, 'embed.js'), 'utf8');
assert.match(embedSrc, /attachShadow/);
const embedPage = readFileSync(distFileFor('/embed/'), 'utf8');
assert.match(embedPage, /https:\/\/takehome\.gautamkhosla\.com\/embed\/v0\.1\.0\/embed\.js/);
assert.match(embedPage, /job board/i);
assert.doesNotMatch(embedPage, /engine\/calculator\.js/);

const leakedFixtures = htmlFiles.filter((file) =>
  /^embed-.+\.html$/.test(path.basename(file)),
);
assert.equal(
  leakedFixtures.length,
  0,
  `Playwright embed-*.html fixtures must not ship in dist:\n${leakedFixtures
    .map((file) => path.relative(DIST, file))
    .join('\n')}`,
);

const publicDir = path.join(ROOT, 'public');
const publicLeaks = readdirSync(publicDir).filter((name) =>
  /^embed-.+\.html$/.test(name),
);
assert.equal(
  publicLeaks.length,
  0,
  `public/ must not contain Playwright fixtures: ${publicLeaks.join(', ')}`,
);

const pdocLeaks = walk(DIST).filter((file) => {
  const rel = path.relative(DIST, file);
  return (
    rel.includes(`${path.sep}pdoc-cache${path.sep}`) ||
    rel.includes(`${path.sep}pdoc-screenshots${path.sep}`) ||
    rel.startsWith(`pdoc-cache${path.sep}`) ||
    rel.startsWith(`pdoc-screenshots${path.sep}`)
  );
});
assert.equal(
  pdocLeaks.length,
  0,
  `PDOC cache/screenshots must not ship in dist:\n${pdocLeaks
    .map((file) => path.relative(DIST, file))
    .join('\n')}`,
);

console.log(
  `site-graph ok: ${pages.length} pages, ${htmlFiles.length} html files, sitemap complete, 0 internal 404s`,
);
