# 构建与发布

## 前端构建

```bash
npm run build
```

该命令会先执行 `vue-tsc --noEmit`，再执行 `vite build`，产物输出到 `dist/`。

## Tauri 构建

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 文档构建

```bash
npm run docs:build
```

产物位于 `docs/.vitepress/dist/`，该目录已加入 `.gitignore`。

## 自动更新配置

`src-tauri/tauri.conf.json` 已启用 `createUpdaterArtifacts`，并配置了 GitHub 与 Gitee 的 `latest.json` 地址。正式发布前需要替换：

```json
"pubkey": "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY"
```

## 版本一致性

发布前检查以下版本是否一致：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

当前项目版本是 `0.1.0`。
