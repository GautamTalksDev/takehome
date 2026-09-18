export const MAX_BATCH = 1000;

export function batchItems(payload) {
  if (Array.isArray(payload)) {
    return payload;
  }
  if (payload && typeof payload === 'object' && Array.isArray(payload.requests)) {
    return payload.requests;
  }
  return null;
}

export function calculationCount(payload) {
  const items = batchItems(payload);
  if (items) {
    return items.length;
  }
  if (payload && typeof payload === 'object') {
    return 1;
  }
  return 1;
}

export function isBatch(payload) {
  return batchItems(payload) != null;
}

export function usageHeaders({ kind, limit, used, reset }) {
  if (kind === 'test' || limit == null) {
    return {
      'x-usage-limit': 'unlimited',
      'x-usage-remaining': 'unlimited',
      'x-usage-reset': reset,
    };
  }
  const remaining = used >= limit ? 0 : limit - used;
  return {
    'x-usage-limit': String(limit),
    'x-usage-remaining': String(remaining),
    'x-usage-reset': reset,
  };
}
