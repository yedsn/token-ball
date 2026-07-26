# 开发环境

TokenBall 是 Tauri 2 桌面应用，前端使用 Vue 3 + Vite，后端使用 Rust。

## 基础依赖

- Node.js 18+，推荐 Node.js 22
- npm
- Rust stable
- Tauri 2 所需系统依赖

Windows 需要：

- Microsoft Edge WebView2 Runtime
- Visual Studio Build Tools，包含 C++ 构建工具和 Windows SDK

macOS 需要：

- Xcode Command Line Tools

## 安装依赖

```bash
npm install
```

## 启动开发

桌面应用：

```bash
npm run tauri:dev
```

仅前端：

```bash
npm run dev
```

Vite dev server 使用 `127.0.0.1:1420`，配置位于 `vite.config.ts` 和 `src-tauri/tauri.conf.json`。

## 文档站点

```bash
npm run docs:dev
```

文档目录是 `docs/`，VitePress 配置位于 `docs/.vitepress/config.mts`。

## 编码注意

仓库包含中文文档、Vue 模板和接口说明。编辑这些文件时使用 UTF-8，避免通过终端乱码输出复制回写中文内容。
