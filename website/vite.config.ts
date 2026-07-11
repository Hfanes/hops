import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  base: "/",
  root: fileURLToPath(new URL(".", import.meta.url)),
  publicDir: "public",
  plugins: [react()],
  build: {
    outDir: "../site-dist",
    emptyOutDir: true,
  },
  server: {
    port: 1422,
    strictPort: true,
  },
  preview: {
    port: 4174,
    strictPort: true,
  },
});
