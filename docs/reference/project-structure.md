# 项目结构

```text
token-ball/
├── src-ui/                 Vue 3 前端
├── src-tauri/              Tauri 2 + Rust 后端
├── docs/                   VitePress 文档站点
├── scripts/release/        发布辅助脚本
├── .github/workflows/      GitHub Actions
├── package.json            npm 脚本和前端依赖
├── vite.config.ts          Vite 配置
└── TokenBall需求.md        早期产品与技术方案
```

## 前端目录

```text
src-ui/src/
├── App.vue                 按 URL 参数切换 orb / hover / main 视图
├── main.ts                 Vue 和 Pinia 入口
├── store.ts                前端状态管理
├── services/tauri.ts       Tauri command 和 window bridge
├── views/OrbView.vue       额度悬浮球
├── views/HoverPanelView.vue 悬停详情面板
├── views/ManagementView.vue 管理窗口
├── types.ts                前端类型
└── styles.css              全局样式
```

## Rust 后端目录

```text
src-tauri/src/
├── lib.rs                  Tauri 应用入口和插件注册
├── app_state.rs            应用共享状态
├── commands/               前端可调用命令
├── providers/              Provider 接入
├── quota/                  额度模型、汇总和关键窗口计算
├── scheduler/              后台同步任务
├── storage/                SQLite 初始化和 repository
├── tray/                   系统托盘
├── events/                 前后端事件
└── windows/                窗口行为
```

## 文档目录

```text
docs/
├── .vitepress/             VitePress 配置和主题
├── public/                 文档站静态资源
├── guide/                  用户说明
├── develop/                开发说明
├── reference/              参考文档
├── index.md                文档站首页
└── qianwen-token-plan-api.md 千问接口说明
```
