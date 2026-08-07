import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  // wasm-bindgen's `--target web` output does its own fetch()/instantiation of the
  // .wasm file; Vite just needs to serve it as a static asset alongside the JS glue.
  assetsInclude: ['**/*.wasm'],
});
