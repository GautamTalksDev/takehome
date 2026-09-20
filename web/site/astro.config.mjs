import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://takehome.gautamkhosla.com',
  output: 'static',
  trailingSlash: 'always',
  prefetch: false,
  vite: {
    build: {
      sourcemap: false,
    },
  },
});
