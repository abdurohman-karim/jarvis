import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import sveltePreprocess from "svelte-preprocess";
import tsconfigPaths from 'vite-tsconfig-paths'
import routify from '@roxi/routify/vite-plugin'

export default defineConfig({
  plugins: [
    svelte({
      preprocess: [
        sveltePreprocess({
          typescript: true,
        }),
      ],
      onwarn: (warning, handler) => {
        const { code, frame } = warning;
        if (code === "css-unused-selector")
            return;

        handler(warning);
      },
    }),
    routify(),
    tsconfigPaths()
  ],

  clearScreen: false,
  // inline (empty) PostCSS config: stops Vite from walking up the tree and
  // picking up a postcss.config.* from outside the project (e.g. $HOME)
  css: {
    postcss: {},
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM == "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});