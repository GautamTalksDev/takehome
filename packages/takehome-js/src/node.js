/**
 * Node entry. Same WASM bytes as the browser build; instantiated from disk
 * so calculate() is synchronous after import (spec §5.4 tests).
 */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import initWasm, {
  initSync,
  calculate as wasmCalculate,
  listJurisdictions as wasmListJurisdictions,
  listRuleSetVersions as wasmListRuleSetVersions,
  engineBuildSha as wasmEngineBuildSha,
} from '../wasm/takehome_wasm.js';

const wasmPath = fileURLToPath(new URL('../wasm/takehome_wasm_bg.wasm', import.meta.url));

initSync({ module: readFileSync(wasmPath) });

export async function init(source) {
  if (source !== undefined) {
    await initWasm(source);
  }
}

export function calculate(requestJson) {
  return wasmCalculate(requestJson);
}

export function listJurisdictions() {
  return wasmListJurisdictions();
}

export function listRuleSetVersions() {
  return wasmListRuleSetVersions();
}

export function engineBuildSha() {
  return wasmEngineBuildSha();
}

export default {
  init,
  calculate,
  listJurisdictions,
  listRuleSetVersions,
  engineBuildSha,
};
