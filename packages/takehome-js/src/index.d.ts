/** Instantiate the embedded WASM module. Required once in the browser and in Workers. */
export function init(
  source?: BufferSource | WebAssembly.Module | URL | string,
): Promise<void>;

/**
 * T4127 deduction. Always returns a JSON string; never throws on malformed input.
 * Success is a Response object. Failure is `{ error: { code, message } }`.
 */
export function calculate(requestJson: string): string;

/** `GET /v1/jurisdictions` JSON, including Quebec as unsupported. */
export function listJurisdictions(): string;

/** Embedded rule-set editions in coverage order. */
export function listRuleSetVersions(): string;

/** Field-by-field comparison of two embedded T4127 editions (spec §12.3). */
export function diffRuleSets(from: string, to: string): string;

/** SHA-256 of the takehome-core source tree. Same value as the native engine. */
export function engineBuildSha(): string;
