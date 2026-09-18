import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld } from './helpers.js';

const DOCS = readFileSync(
  path.join(path.dirname(fileURLToPath(import.meta.url)), '../../../docs/year-projection.md'),
  'utf8',
);

const CPP_MAX = '4230.45';
const EI_MAX = '1123.07';
const YMPE = '74600.00';
const EXEMPTION_27 = '129.62';
const EXEMPTION_53 = '66.03';

const HIGH_ON = {
  as_of: '2026-01-01',
  province: 'ON',
  pay_period: 26,
  gross_pay: '4000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
};

const LOW_ON = { ...HIGH_ON, gross_pay: '2000.00' };

function cents(token) {
  const neg = String(token).startsWith('-');
  const raw = neg ? String(token).slice(1) : String(token);
  const [dollars, frac = '00'] = raw.split('.');
  const value =
    Number.parseInt(dollars, 10) * 100 + Number.parseInt((frac + '00').slice(0, 2), 10);
  return neg ? -value : value;
}

async function year(body, world = createWorld()) {
  return call(
    'POST',
    '/v1/deductions/year',
    body,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
}

test('22. 26 biweekly periods at constant gross: CPP sums to the annual maximum and the cap period is named', async () => {
  const { status, json } = await year(HIGH_ON);
  assert.equal(status, 200);
  assert.equal(json.periods.length, 26);
  assert.equal(json.totals.cpp, CPP_MAX);
  assert.equal(typeof json.cpp_cap_period, 'number');
  assert.ok(json.cpp_cap_period >= 1 && json.cpp_cap_period <= 26);
  const cap = json.periods[json.cpp_cap_period - 1];
  assert.ok(cents(cap.response.employee.cpp) > 0);
  for (let i = json.cpp_cap_period; i < 26; i += 1) {
    assert.equal(json.periods[i].response.employee.cpp, '0.00', `period ${i + 1} after CPP cap`);
  }
});

test('23. 26 biweekly periods at constant gross: EI sums to the annual maximum and the cap period is named', async () => {
  const { status, json } = await year(HIGH_ON);
  assert.equal(status, 200);
  assert.equal(json.totals.ei, EI_MAX);
  assert.equal(typeof json.ei_cap_period, 'number');
  assert.ok(json.ei_cap_period >= 1 && json.ei_cap_period <= 26);
  assert.ok(cents(json.periods[json.ei_cap_period - 1].response.employee.ei) > 0);
  for (let i = json.ei_cap_period; i < 26; i += 1) {
    assert.equal(json.periods[i].response.employee.ei, '0.00', `period ${i + 1} after EI cap`);
  }
});

test('24. CPP2 starts in the YMPE-crossing period and is zero before — spec §21.5', async () => {
  const { status, json } = await year(HIGH_ON);
  assert.equal(status, 200);
  assert.equal(typeof json.cpp2_start_period, 'number');
  assert.equal(json.cpp2_start_period, json.ympe_cross_period);
  for (let i = 0; i < json.cpp2_start_period - 1; i += 1) {
    assert.equal(json.periods[i].response.employee.cpp2, '0.00', `period ${i + 1} before CPP2`);
  }
  assert.ok(cents(json.periods[json.cpp2_start_period - 1].response.employee.cpp2) > 0);
});

test('25. a salary that crosses YMPE mid-year names the crossing period', async () => {
  const { status, json } = await year(HIGH_ON);
  assert.equal(status, 200);
  assert.equal(typeof json.ympe_cross_period, 'number');
  const i = json.ympe_cross_period - 1;
  const before = json.periods[i].ytd_pensionable_earnings_before;
  const after = json.periods[i].ytd_pensionable_earnings_after;
  assert.ok(cents(before) <= cents(YMPE), `PE before period ${json.ympe_cross_period} is ${before}`);
  assert.ok(cents(after) > cents(YMPE), `PE after period ${json.ympe_cross_period} is ${after}`);
});

test('26. a salary that never reaches YMPE has CPP2 of zero in all 26 periods', async () => {
  const { status, json } = await year(LOW_ON);
  assert.equal(status, 200);
  assert.equal(json.periods.length, 26);
  assert.equal(json.ympe_cross_period, null);
  assert.equal(json.cpp2_start_period, null);
  assert.equal(json.totals.cpp2, '0.00');
  for (const period of json.periods) {
    assert.equal(period.response.employee.cpp2, '0.00');
    assert.ok(cents(period.ytd_pensionable_earnings_after) < cents(YMPE));
  }
});

test('27. a BC year that crosses 2026-06-30 / 2026-07-01 uses different rules per period', async () => {
  const { status, json } = await year({ ...HIGH_ON, province: 'BC' });
  assert.equal(status, 200);
  const versions = json.periods.map((p) => p.response.rule_set_version);
  assert.ok(versions.includes('2026-01-01'));
  assert.ok(versions.includes('2026-07-01'));
  const lastJan = json.periods.filter((p) => p.response.rule_set_version === '2026-01-01').at(-1);
  const firstJul = json.periods.find((p) => p.response.rule_set_version === '2026-07-01');
  assert.ok(lastJan.as_of <= '2026-06-30', lastJan.as_of);
  assert.ok(firstJul.as_of >= '2026-07-01', firstJul.as_of);
  assert.equal(lastJan.response.rule_set_version, '2026-01-01');
  assert.equal(firstJul.response.rule_set_version, '2026-07-01');
  assert.notEqual(
    lastJan.response.employee.provincial_tax,
    firstJul.response.employee.provincial_tax,
  );
});

test('28. sum of period federal tax is within the documented per-period rounding tolerance of annual T1', async () => {
  const { status, json } = await year(LOW_ON);
  assert.equal(status, 200);
  assert.match(DOCS, /per-period rounding/i);
  assert.match(DOCS, /one cent per pay period/i);
  assert.match(DOCS, /T1/);
  const t1 = json.annual_t1;
  const sum = json.totals.federal_tax;
  const tolerance = json.federal_tax_sum_tolerance;
  assert.equal(tolerance, '0.26');
  const gap = Math.abs(cents(sum) - cents(t1));
  assert.ok(
    gap <= cents(tolerance),
    `federal tax sum ${sum} vs annual T1 ${t1} gap ${gap} cents, tolerance ${tolerance}`,
  );
});

test('29. 53-week and 27-biweekly years produce that many periods with the Chapter 6 basic exemption', async () => {
  const weekly = await year({ ...LOW_ON, pay_period: 53, gross_pay: '1000.00' });
  assert.equal(weekly.status, 200);
  assert.equal(weekly.json.periods.length, 53);
  for (const period of weekly.json.periods) {
    assert.equal(period.cpp_basic_exemption, EXEMPTION_53);
    assert.equal(period.response.breakdown.P, '53');
  }

  const biweekly = await year({ ...LOW_ON, pay_period: 27, gross_pay: '2000.00' });
  assert.equal(biweekly.status, 200);
  assert.equal(biweekly.json.periods.length, 27);
  for (const period of biweekly.json.periods) {
    assert.equal(period.cpp_basic_exemption, EXEMPTION_27);
    assert.equal(period.response.breakdown.P, '27');
  }
});
