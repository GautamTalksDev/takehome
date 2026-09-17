import source from './assets/plans.json' with { type: 'json' };

export const PLANS = source;

export function livePlan(id) {
  return PLANS.live.find((plan) => plan.id === id) ?? null;
}

export function cadLabel(cents) {
  if (cents === 0) {
    return '$0 CAD';
  }
  const dollars = String((cents - (cents % 100)) / 100);
  const remainder = cents % 100;
  if (remainder === 0) {
    return `$${dollars} CAD`;
  }
  return `$${dollars}.${String(remainder).padStart(2, '0')} CAD`;
}

export function periodStart(isoDate) {
  return `${isoDate.slice(0, 7)}-01`;
}

export function periodReset(isoDate) {
  const year = Number.parseInt(isoDate.slice(0, 4), 10);
  const month = Number.parseInt(isoDate.slice(5, 7), 10);
  if (month === 12) {
    return `${year + 1}-01-01T00:00:00Z`;
  }
  return `${year}-${String(month + 1).padStart(2, '0')}-01T00:00:00Z`;
}
