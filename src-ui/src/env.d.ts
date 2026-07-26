/// <reference types="vite/client" />

// 由 vite.config.ts 的 define 在构建期注入，值为 package.json 的 version。
declare const __APP_VERSION__: string;

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
