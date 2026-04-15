import { fileURLToPath, URL } from "node:url";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

const webRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("..", import.meta.url));

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, repoRoot, "");
  const apiProxyTarget =
    env.DEPLOYWERK_API_PROXY?.trim() || "http://127.0.0.1:8080";

  return {
    // Load `.env` from repo root so `VITE_*` matches the API `.env`.
    envDir: repoRoot,
    plugins: [react()],
    resolve: {
      alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
    },
    root: webRoot,
    server: {
      port: 5173,
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    preview: {
      port: 4173,
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (!id.includes("node_modules")) return;
            // Stable vendor chunks for long cache; paths work on Windows and POSIX.
            if (
              /[\\/]node_modules[\\/](react-dom|react-router-dom)([\\/]|$)/.test(
                id,
              ) ||
              /[\\/]node_modules[\\/]react[\\/]/.test(id)
            ) {
              return "vendor-react";
            }
            if (/[\\/]node_modules[\\/]lucide-react[\\/]/.test(id)) {
              return "vendor-icons";
            }
          },
        },
      },
    },
  };
});
