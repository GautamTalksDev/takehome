# Offline and self-host

**Who this is for:** Teams that need T4127 (Payroll Deductions Formulas) math without sending employee payloads to Takehome Cloud, or who want the same engine in-browser.

**When you finish:** You can choose between hosted API, embedded WASM, and native Rust, and you know how rule updates stay integrity-checked.

A **claim code** in the JSON request maps TD1 boxes to credits the same way as the hosted API.

<figure>
  <img src="/diagrams/06-one-engine-every-runtime.svg" alt="One engine, every runtime" />
  <figcaption>The same Rust core compiles to native and to WASM. Browser, Node, Python, and the Worker share engine_build_sha256. The browser path makes no network call after load.</figcaption>
</figure>

## One engine, every runtime

The `takehome-core` crate is the source of truth. Cloudflare Workers, the public API, Node (`takehome-ca` on npm), and browser calculators call the same logic. Matching releases share one `engine_build_sha256` on the response envelope.

WASM embeds rule tables in the module. After `init()` or `initSync()`, `calculate(requestJson)` needs no network I/O. Employee gross, **claim code**, and YTD never leave the device when you run locally.

```javascript
import { init, calculate, engineBuildSha } from 'takehome-ca';

await init();

const response = JSON.parse(
  calculate(
    JSON.stringify({
      as_of: '2026-01-15',
      province: 'ON',
      pay_period: 52,
      gross_pay: '1000.00',
      federal_claim_code: 1,
      provincial_claim_code: 1,
    }),
  ),
);
console.log(response.employee.net_pay, engineBuildSha());
```

See [packages/takehome-js/README.md](../packages/takehome-js/README.md) for Node vs browser loaders and Cloudflare Worker wiring.

## Browser calculator privacy

The hosted Ontario calculator and similar pages can load WASM once, then compute entirely client-side. No POST to `/v1/deductions` is required for that mode. This fits HR demos, employee self-service on locked-down networks, and air-gapped kiosks after the wasm bytes cache.

## Signed rule updates

CRA PDFs and parsed rule JSON pass through a pipeline: archive with content hash, parse, sign, verify before load. July editions overlay January tables rather than replacing unrelated jurisdictions. Offline bundles inherit the same verification when you refresh embedded rules in a new engine release.

Operators who self-host the API Worker still pull rule data from the signed artifacts shipped with the engine build. Custom rule injection without rebuild is not supported.

## When to choose offline

| Need | Prefer |
| --- | --- |
| Pay run integration, webhooks, centralized metering | Hosted API with `np_live_` keys |
| CI golden files, unlimited replays | `np_test_` keys or WASM in tests |
| Employee data must not leave VPC or browser | WASM or native Rust in your service |
| Mobile or desktop app without backend | WASM with embedded rules |
| Automatic rule-set change alerts | Hosted API plus [webhooks.md](./webhooks.md) |

Self-hosting the Worker (`services/api`, `npm run local`) mirrors production routes for development. Production compliance record: [CONFORMANCE.md](../CONFORMANCE.md) and `GET /v1/conformance`.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
