import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';

const cargoToml = readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf-8');
const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
const appVersion = match ? match[1] : '0.0.0';

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(appVersion),
  },
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8080'
    }
  }
});
