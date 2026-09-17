# Static T4127 calculator (Cloudflare Pages)

```bash
cd web/site
npm ci
npm test
```

Build output is `dist/`. Pages build command: `bash web/site/scripts/sync-engine.sh && cd web/site && npm ci && npm run build`. Output directory: `web/site/dist`.

System font, one accent colour, no account, no cookies, no modal. The WASM module is the engine; salary never leaves the browser.
