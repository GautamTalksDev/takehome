import source from '../data/plans.json';

export const PLANS = source;

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
