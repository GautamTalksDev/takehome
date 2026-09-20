export function parseHeadersFile(text) {
  const blocks = [];
  let current = null;
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) {
      continue;
    }
    if (!line.startsWith(' ') && !line.startsWith('\t')) {
      current = { path: trimmed, headers: {} };
      blocks.push(current);
      continue;
    }
    if (!current) {
      continue;
    }
    const idx = trimmed.indexOf(':');
    if (idx > 0) {
      current.headers[trimmed.slice(0, idx).trim()] = trimmed
        .slice(idx + 1)
        .trim();
    }
  }
  return blocks;
}

export function blockFor(blocks, pathName) {
  const found = blocks.find((block) => block.path === pathName);
  if (!found) {
    throw new Error(`no _headers block for ${pathName}`);
  }
  return found;
}
