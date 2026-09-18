/**
 * Canonical T4127 source bytes (spec §12.3).
 *
 * HTML: UTF-8, LF newlines, BOM stripped, CRA "Date modified" / dcterms.modified
 * chrome removed. The rest of the page is kept so a rate change shows up in
 * the unified diff, not only a hash flip.
 *
 * CSV: UTF-8, LF newlines, BOM stripped. Bytes otherwise untouched.
 */

export function canonicalize(kind, input) {
  const text = decode(input);
  if (kind === 'csv') {
    return canonicalizeCsv(text);
  }
  return canonicalizeHtml(text);
}

export function canonicalizeHtml(text) {
  const lf = stripBom(text).replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  return lf
    .replace(/<p>\s*Date modified:[^<]*<\/p>\s*/gi, '')
    .replace(/^\s*Date modified:[^\n]*\n/gim, '')
    .replace(/<meta[^>]*dcterms\.modified[^>]*>\s*/gi, '')
    .replace(/<time[^>]*property=["']dcterms:modified["'][^>]*>[^<]*<\/time>/gi, '')
    .replace(/\n{3,}/g, '\n\n');
}

export function canonicalizeCsv(text) {
  return stripBom(text).replace(/\r\n/g, '\n').replace(/\r/g, '\n');
}

function stripBom(text) {
  return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
}

function decode(input) {
  if (typeof input === 'string') {
    return input;
  }
  if (input instanceof ArrayBuffer) {
    return new TextDecoder('utf-8').decode(input);
  }
  if (ArrayBuffer.isView(input)) {
    return new TextDecoder('utf-8').decode(input);
  }
  return String(input);
}

export function csvHrefs(html, baseUrl) {
  const found = [];
  const re = /href\s*=\s*["']([^"']+\.csv[^"']*)["']/gi;
  let match;
  while ((match = re.exec(html)) !== null) {
    found.push(new URL(match[1], baseUrl).href);
  }
  return [...new Set(found)].sort();
}
