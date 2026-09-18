# Static T4127 calculator (Cloudflare Pages)

```bash
cd web/site
npm ci
npm test
```

Build output is `dist/`. Pages build command: `bash web/site/scripts/sync-engine.sh && cd web/site && npm ci && npm run build`. Output directory: `web/site/dist`.

Deploy (Pages project `takehome`, Worker custom domain on `takehome.gautamkhosla.com`):

```bash
cd web/site && npm run build
npx wrangler pages deploy dist --project-name takehome --commit-dirty=true
cd ../../services/api && npm run deploy
node ../../scripts/smoke-live.mjs
```

The Worker serves `/v1*` and `/health`; the Pages build in `web/site/dist` is the rest of the origin. Smoke is against the live hostname, not localhost.

System font, one accent colour, no account, no cookies, no modal. The WASM module is the engine; salary never leaves the browser.

System font, one accent colour, no account, no cookies, no modal. The WASM module is the engine; salary never leaves the browser.
