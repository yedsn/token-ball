# 常用命令

## 应用开发

| 命令 | 用途 |
| --- | --- |
| `npm install` | 安装前端和文档依赖 |
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run tauri:dev` | 启动 Tauri 桌面开发模式 |
| `npm run build` | 类型检查并构建前端 |
| `npm run typecheck` | 只执行 TypeScript 类型检查 |
| `npm run tauri:build` | 构建桌面安装包 |

## 文档站点

| 命令 | 用途 |
| --- | --- |
| `npm run docs:dev` | 本地启动 VitePress 文档站 |
| `npm run docs:build` | 构建静态文档站 |
| `npm run docs:preview` | 预览文档构建产物 |

## Rust 检查

在 `src-tauri/` 目录下执行：

```bash
cargo check
```

如果本机 Cargo registry 网络不稳定，并且已有完整锁文件和缓存，可以尝试：

```bash
cargo check --offline
```
