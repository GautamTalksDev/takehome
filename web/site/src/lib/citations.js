/**
 * Citation labels for the calculator: factor, what it is, which T4127 table,
 * and the CRA heading fragment for a deep link.
 */

const TABLE_FRAGMENT = [
  ['Table 8.1', 'toc64'],
  ['Table 8.2', 'toc65'],
  ['Table 8.29', 'toc93'],
  ['Chapter 2', 'toc11'],
  ['Chapter 6', 'toc46'],
  ['Chapter 7', 'toc61'],
  ['Chapter 8', 'toc63'],
  ['Step 1', 'toc21'],
  ['Step 2', 'toc22'],
  ['Step 3', 'toc23'],
  ['Step 4', 'toc25'],
  ['Step 5', 'toc26'],
  ['Step 6', 'toc36'],
  ['Chapter 4', 'toc20'],
];

export function shortWhat(definition) {
  const clause = definition.replace(/\.$/, '').split(/ that | for the pay | for the year/)[0];
  return clause.split(',')[0].trim();
}

export function shortRef(reference) {
  return reference.replaceAll('Chapter ', 'Ch. ').replace('; ', ', ');
}

export function citationFragment(reference) {
  for (const [needle, hash] of TABLE_FRAGMENT) {
    if (reference.includes(needle)) {
      return hash;
    }
  }
  return '';
}

export function citationIndex(factorRows) {
  const out = {};
  for (const row of factorRows) {
    out[row.symbol] = {
      what: shortWhat(row.definition),
      table: shortRef(row.t4127_reference),
      hash: citationFragment(row.t4127_reference),
    };
  }
  return out;
}

export function citationHref(sourceUrl, hash) {
  const base = String(sourceUrl || '').split('#')[0];
  if (!hash) {
    return base;
  }
  return `${base}#${hash}`;
}
