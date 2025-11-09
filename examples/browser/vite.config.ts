import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
// Resolve to repository root (go up 2 levels from examples/browser)
const repoRoot = path.resolve(__dirname, "../..");

export default defineConfig({
  plugins: [react(), wasm(), topLevelAwait()],
  optimizeDeps: {
    exclude: ["sha3-wgpu"],
  },
  server: {
    fs: {
      // Allow access to repository root (where pkg directory is located)
      allow: [repoRoot],
    },
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
});
