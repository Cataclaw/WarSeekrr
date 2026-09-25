import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// Fixed port and host for `tauri dev`.
export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
