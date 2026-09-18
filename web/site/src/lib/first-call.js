export const FIRST_CALL = {
  as_of: '2026-01-15',
  province: 'ON',
  pay_period: 52,
  gross_pay: '1000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
};

export const FIRST_CALL_NET_PAY = '800.79';

export const FIRST_CALL_JSON = JSON.stringify(FIRST_CALL);

export const FIRST_CALL_RESPONSE = `{
  "rule_set_version": "2026-01-01",
  "employee": {
    "federal_tax": "81.61",
    "provincial_tax": "45.80",
    "cpp": "55.50",
    "cpp2": "0.00",
    "ei": "16.30",
    "total_deductions": "199.21",
    "net_pay": "800.79"
  }
}`;

export function firstCallCurl(apiOrigin, key) {
  const host = apiOrigin.replace(/\/$/, '');
  const token = key ?? 'np_test_YOUR_KEY';
  return `curl -sS -X POST ${host}/v1/deductions \\
  -H 'content-type: application/json' \\
  -H 'authorization: Bearer ${token}' \\
  -d '${FIRST_CALL_JSON}'`;
}
