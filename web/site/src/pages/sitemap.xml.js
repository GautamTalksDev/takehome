import { htmlPaths } from '../lib/catalog.js';

export function GET() {
  const urls = htmlPaths()
    .map((path) => `  <url><loc>https://takehome.gautamkhosla.com${path}</loc></url>`)
    .join('\n');
  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
  return new Response(body, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8' },
  });
}
