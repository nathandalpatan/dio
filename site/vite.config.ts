import { defineConfig } from 'vite';
import { fileURLToPath, URL } from 'node:url';

// The site has two pages: the home page and the changelog.
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        changelog: fileURLToPath(new URL('./changelog/index.html', import.meta.url)),
      },
    },
  },
});
