import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loadEngine } from './helpers.js';

test('malformed JSON returns a structured error and does not throw', async () => {
  const engine = await loadEngine();
  for (const input of ['', '{', 'null', 'not json', '\u0000']) {
    let body;
    try {
      body = engine.calculate(input);
    } catch (err) {
      assert.fail(`calculate(${JSON.stringify(input)}) threw: ${err}`);
    }
    const parsed = JSON.parse(body);
    assert.equal(parsed.error.code, 'malformed_json', input);
    assert.equal(typeof parsed.error.message, 'string');
    assert.ok(parsed.error.message.length > 0);
    assert.equal(parsed.employee, undefined);
  }
});

test('listJurisdictions and listRuleSetVersions are JSON', async () => {
  const engine = await loadEngine();
  const jurisdictions = JSON.parse(engine.listJurisdictions());
  const qc = jurisdictions.jurisdictions.find((row) => row.code === 'QC');
  assert.equal(qc.supported, false);
  const versions = JSON.parse(engine.listRuleSetVersions());
  assert.deepEqual(
    versions.rule_set_versions.map((row) => row.version),
    ['2026-01-01', '2026-07-01'],
  );
});
