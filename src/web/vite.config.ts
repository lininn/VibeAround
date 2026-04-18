import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "path";
import { defineConfig } from "vite";

export default defineConfig({
  base: "/va/",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@va/client": path.resolve(__dirname, "../shared/client-ts/src/index.ts"),
    },
  },
  server: {
    port: 5180,
    strictPort: true,
  },
});
