/**
 * Browser / Cloudflare Worker entry. One wasm-bindgen `web` build.
 * Call `await init()` (browser) or `await init(wasmModule)` (Worker).
 */

import initWasm, {
  calculate as wasmCalculate,
  listJurisdictions as wasmListJurisdictions,
  listRuleSetVersions as wasmListRuleSetVersions,
  engineBuildSha as wasmEngineBuildSha,
} from '../wasm/takehome_wasm.js';

let initialized = false;

export async function init(source) {
  if (initialized && source === undefined) {
    return;
  }
  await initWasm(source);
  initialized = true;
}

function requireInit() {
  if (!initialized) {
    throw new Error('takehome-ca: call await init() before calculate()');
  }
}

export function calculate(requestJson) {
  requireInit();
  return wasmCalculate(requestJson);
}

export function listJurisdictions() {
  requireInit();
  return wasmListJurisdictions();
}

export function listRuleSetVersions() {
  requireInit();
  return wasmListRuleSetVersions();
}

export function engineBuildSha() {
  requireInit();
  return wasmEngineBuildSha();
}

export default {
  init,
  calculate,
  listJurisdictions,
  listRuleSetVersions,
  engineBuildSha,
};
