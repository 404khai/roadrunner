import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    plugins: [react()],
    server: {
      proxy: {
        "/v1": {
          target: env.ROADRUNNER_API_URL || "http://127.0.0.1:3000",
          changeOrigin: false,
          ...(env.ROADRUNNER_API_TOKEN
            ? {
                headers: {
                  Authorization: `Bearer ${env.ROADRUNNER_API_TOKEN}`,
                },
              }
            : {}),
        },
      },
    },
  };
});
