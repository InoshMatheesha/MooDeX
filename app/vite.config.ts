// vite.config.ts - Frontend lives in ui/; output goes to ui/dist for Tauri.
import { defineConfig } from "vite";

export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
