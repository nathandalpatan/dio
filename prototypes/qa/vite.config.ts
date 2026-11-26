import { defineConfig } from 'vite';
import { fileURLToPath, URL } from 'node:url';
import tailwindcss from '@tailwindcss/vite';

const root = fileURLToPath(new URL('.', import.meta.url));
const stub = fileURLToPath(new URL('./tauri-stub.ts', import.meta.url));

export default defineConfig({
  root,
  plugins: [tailwindcss()],
  resolve: { alias: { '@tauri-apps/api/core': stub, '@tauri-apps/api/event': stub } },
  server: { port: 1421, strictPort: true },
  clearScreen: false,
});
