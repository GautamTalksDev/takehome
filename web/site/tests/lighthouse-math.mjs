/**
 * Pure helpers for Lighthouse multi-run scoring.
 * Median of an odd-length list is the middle value after sort.
 */
export function median(values) {
  if (!Array.isArray(values) || values.length === 0) {
    throw new Error('median requires a non-empty array');
  }
  const sorted = [...values].map(Number).sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) {
    return sorted[mid];
  }
  return (sorted[mid - 1] + sorted[mid]) / 2;
}

export function lighthouseMode(env = process.env) {
  if (env.TAKEHOME_LIGHTHOUSE_MODE === 'local' || env.TAKEHOME_LIGHTHOUSE_MODE === 'ci') {
    return env.TAKEHOME_LIGHTHOUSE_MODE;
  }
  return env.GITHUB_ACTIONS === 'true' ? 'ci' : 'local';
}
