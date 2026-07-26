# 站点与发布说明

TokenBall 文档站使用 VitePress，结构参考 OpenDock 项目，但内容按 TokenBall 的 Provider 和额度球工作流重新组织。

## 本地开发

```bash
npm run docs:dev
```

## 本地构建

```bash
npm run docs:build
```

构建输出目录：

```text
docs/.vitepress/dist
```

## GitHub Pages

站点配置位于 `docs/.vitepress/config.mts`。`base` 会按环境自动计算：

| 环境 | base |
| --- | --- |
| 本地开发 | `/` |
| GitHub Actions | `/<repo-name>/` |
| 显式覆盖 | 使用 `DOCS_BASE` 环境变量 |

GitHub Pages 工作流位于 `.github/workflows/pages.yml`。仓库 Settings -> Pages 中需要选择 GitHub Actions 作为发布来源。

## 静态资源

文档站图标位于：

```text
docs/public/logo.png
docs/public/favicon.png
```

这两个文件来自 `src-tauri/icons/`，如果应用图标更新，建议同步更新文档站图标。
