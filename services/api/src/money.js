/**
 * Lexical money as integer cents. Never IEEE-754.
 * "12.34" → 1234. Construction is parseInt on digit strings only.
 */

export function moneyToCents(token) {
  const text = String(token);
  const neg = text.startsWith('-');
  const raw = neg ? text.slice(1) : text;
  const [dollars, frac = '00'] = raw.split('.');
  const value =
    Number.parseInt(dollars, 10) * 100 +
    Number.parseInt((frac + '00').slice(0, 2), 10);
  return neg ? -value : value;
}

export function centsToMoney(cents) {
  const neg = cents < 0;
  const abs = cents < 0 ? -cents : cents;
  const dollars = String(Math.trunc(abs / 100));
  const frac = String(abs % 100).padStart(2, '0');
  return `${neg ? '-' : ''}${dollars}.${frac}`;
}

export function addMoney(a, b) {
  return centsToMoney(moneyToCents(a) + moneyToCents(b));
}

export function cmpMoney(a, b) {
  const left = moneyToCents(a);
  const right = moneyToCents(b);
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}
