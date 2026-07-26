import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const pkgVersion = JSON.parse(
  readFileSync(fileURLToPath(new URL("./package.json", import.meta.url)), "utf-8")
).version as string;

export default defineConfig({
  plugins: [vue()],
  define: {
    // 构建期注入 package.json 版本号，前端直接读 __APP_VERSION__，
    // 不依赖运行时 invoke，避免旧二进制未注册命令时版本号读不到。
    __APP_VERSION__: JSON.stringify(pkgVersion)
  },
  root: "src-ui",
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2020"
  }
});
