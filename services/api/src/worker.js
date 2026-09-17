import { init, calculate, listJurisdictions, listRuleSetVersions, engineBuildSha } from 'takehome-ca';
import wasm from 'takehome-ca/takehome_wasm_bg.wasm';
import { handle } from './handler.js';

const ready = init(wasm);

const engine = {
  async calculate(requestJson) {
    await ready;
    return calculate(requestJson);
  },
  async listJurisdictions() {
    await ready;
    return listJurisdictions();
  },
  async listRuleSetVersions() {
    await ready;
    return listRuleSetVersions();
  },
  async engineBuildSha() {
    await ready;
    return engineBuildSha();
  },
};

export default {
  async fetch(request, env) {
    return handle(request, env, engine);
  },
};
