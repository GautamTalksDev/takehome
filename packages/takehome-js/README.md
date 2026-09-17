Canadian payroll deduction formulas (CRA T4127), compiled to WebAssembly.

One engine: the JSON that comes out of `calculate` here is byte-identical
to the native Rust crate. Rule tables are embedded in the module, so a
calculation does not touch the network (spec §5.5).

```js
import { init, calculate, engineBuildSha } from 'takehome-ca';

await init(); // Node does this from disk; browsers fetch the wasm bytes.

const response = JSON.parse(
  calculate(
    JSON.stringify({
      as_of: '2026-01-15',
      province: 'ON',
      pay_period: 52,
      gross_pay: '1000.00',
    }),
  ),
);
console.log(response.employee.net_pay, engineBuildSha());
```

`calculate` always returns a JSON string. Malformed input is
`{"error":{"code":"malformed_json","message":"..."}}` — it does not throw.

One wasm-bindgen `web` build. Two JS loaders, because Node has `fs` and
Workers do not:

- **Node** (`src/node.js`) — `initSync` from disk; `calculate` is then sync.
- **Browser** (`src/browser.js`) — `await init()` fetches the wasm bytes.
- **Cloudflare Worker** — same browser loader; pass the module in:

```js
import { init, calculate } from 'takehome-ca';
import wasm from 'takehome-ca/takehome_wasm_bg.wasm';

await init(wasm);
```

## Surface

- `calculate(requestJson) -> string`
- `listJurisdictions() -> string`
- `listRuleSetVersions() -> string`
- `engineBuildSha() -> string`

The request/response contract is the same as the HTTP API / native engine.

Publish (once `npm login` works): `npm publish --access public` from this directory.
