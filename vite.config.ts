/// <reference types="vitest/config" />
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// No @types/node in this project (browser-only front end); declare just the
// slice of `process` this file touches rather than pulling in the full dep.
declare const process: { env: Record<string, string | undefined> };

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Windows: cargo writes/locks target/debug/deps/*.dll while compiling;
    // without this, Vite's fs watcher throws EBUSY and kills the dev server.
    watch: { ignored: ['**/src-tauri/**'] },
  },
  // Svelte 5 ships separate "browser" (client) and default (SSR) builds via
  // package.json export conditions. Vitest needs the browser condition or
  // component mounts hit the SSR build and throw `lifecycle_function_unavailable`.
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: {
    environment: 'jsdom',
    setupFiles: ['./vitest-setup.ts'],
  },
});
