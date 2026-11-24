# 验收记录

## 结果与范围

2026-11-23 至 2026-11-24，本次实现位于 TokenBall；PhoneBox 仓库无修改，未修改现用 PhoneBox 配置、推送值或安装状态，未重启其进程。实现与自动化检查完成，真实 Agent 整体联调（任务 6.2）保留待验收，不据此归档变更。

## 已执行检查

| 检查 | 结果与边界 |
| --- | --- |
| `cargo test --offline --lib --manifest-path src-tauri/Cargo.toml` | 50/50 通过，含 13 个 PhoneBox 配置/映射/发送测试；既有真实 Provider 测试未启用在线凭据开关，不代表平台在线验收 |
| `npm run typecheck` | 通过 |
| `npm run build` | 通过；也在 Tauri 构建前再次执行成功 |
| `npm run test:quota-period` | 11/11 通过，既有周期识别、独立恢复时间与桌面规则回归 |
| `npm run test:phonebox` | 6/6 通过，含导入格式、内嵌一致性、可控时间、恶意文本及事件接入静态检查 |
| `npm run phonebox:check` | 通过，仓库和文档下载 JSON 一致 |
| `npm run test:phonebox-browser` | 通过，真实 Vue 页面配合模拟桌面桥，检查未启用/未保存禁止立即推送、保存后可推送、修改后禁用，以及坏配置默认值修复 |
| 原版 HTML 组件浏览器检查 | 通过；只读加载 PhoneBox `1836f00` 的 `W.html` 和 `HTML_PRELUDE`，运行其原版 allow-scripts 沙箱，检查 3 账号、缺失周期、恶意文本、横竖屏、零额外网络请求；不是整个 Agent 的 Live preview |
| `npm run docs:build` | 通过 |
| Windows 安装包资源检查 | Debug NSIS 包成功，禁用签名和 updater 产物；生成的 installer.nsi 明确包含 integrations/phonebox 两个文件，未安装或发布 |
| 窗口生命周期检查 | 后台发送者由 Rust 应用启动，与 Vue 生命周期、窗口对象和显隐无依赖；现有关闭管理窗口只隐藏窗口；无窗口的后台测试可持续心跳。未做原生桌面逐按钮人工验收 |

发送模拟服务覆盖固定路径与 TTL、2xx 确认、非法 JSON/错误 key、重定向、HTTP 500、超时、短心跳恢复、切换目标取消、禁用取消和连续信号合并。使用毫秒级测试周期，不等待真实 60 秒心跳。

映射测试覆盖多 Provider、停用实例、平台异常时保留缓存、缺失采集时间、已知额度平均、全未知 null、独立恢复时间、随机延迟、敏感字段哨兵和 303 账号截断。设置测试包含实际临时 SQLite 文件关闭并重开；测试后删除该测试文件，不涉及用户数据库。

NSIS 检查命令：

```powershell
npm run tauri:build -- --debug --bundles nsis --no-sign --ci --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

浏览器检查需要可用 Playwright，可使用已有运行时而不向项目增加依赖：

```powershell
$env:PLAYWRIGHT_MODULE='<已有 Playwright 模块路径>'
$env:PLAYWRIGHT_EXECUTABLE='<已有 Chromium 可执行文件路径>'
$env:PHONEBOX_WIDGET_SOURCE='<PhoneBox 仓库>/android/app/src/main/assets/www/widgets/pc.js'
npm run test:phonebox-browser
```

不设置 PHONEBOX_WIDGET_SOURCE 时只使用模拟桥沙箱，不能记为原版组件检查。截图在忽略目录 `.diagnostics/phonebox/`。

初版资源 JSON 的 SHA-256（初次打包时仓库、文档下载、构建资源三处相同；下方样式更新后安装包尚未重打）：

`4FF390912DC8DA7EF91118802137F617E31CBACF5786E82BFC2871C5F8C9AF0B`

## 未执行的实际 Agent / 手机验收

当前源码提交 `1836f00`，项目版本 0.9.29；安装的 PhoneBoxAgent.exe 产品版本为 `0.9.28+bf74b03079f7071ac0060eeca3f0162bca28272c`。两者不能混称为同一已验证版本。

任务 6.2 未完成的原因：

- PhoneBox `Program.cs` 虽支持 `--config`，但注册的 `PushStore` 默认写 `AppPaths.ConfigDir/data.json`，并未按该参数隔离。另开端口和配置可能仍覆盖现用 tokenball 或其他推送值。
- 当前仅安装 .NET SDK 9.0.317，源码需要 .NET 10，未安装新 SDK 或修改 PhoneBox 以绕过隔离问题。
- 未对现用 Agent 发写请求、导入页面、重启进程，也未把模拟服务的恢复/模板时钟测试声称为真实 Agent 的 TTL 验收。

后续需在独立 Windows 用户、隔离虚拟机，或用户明确授权的实际环境中执行：推送并检查脱敏 JSON；通过 Pages → Import 追加并保存；验证 Live preview；重启 Agent 后验证恢复；关闭推送后等待最多 180 秒确认 key 消失。

未连接或操作手机；USB 与 Wi-Fi 真机账号列表、周期恢复、缓存提示均未实测。浏览器横竖屏截图不等于手机验收。任务 6.3 仅按“明确记录未实测项”的分支完成。

## 2026-11-24 圆环样式更新

总体余量改为大字号主圆环，保留平均关键余量计算说明；各账号的 5 小时、周、月改为独立圆环，低余量使用琥珀/红色，未知值使用虚线环与占位符，不表示为 0%。横屏左右分栏、竖屏上下排列；窄屏与矮横屏保留可滚动账号列表。

- 模板测试 7/7 通过，新增圆环百分比、阈值颜色与未知值测试。
- 原版 HTML 组件沙箱浏览器检查通过；检查 420×780、900×430、320×568、900×320，包含主字号、圆环数量、横向无溢出、总览可见、账号区可滚动及零网络请求。
- 已重新生成仓库与文档下载 JSON；一致性检查通过。此次仅更新模板/生成资源/测试，未改变后端协议或推送逻辑，未更新现用 PhoneBox，也未重新打包安装程序。
- 该次更新仓库与文档 JSON SHA-256：B9B50E831B8F82F0C69F0B4A9D634C2B89424688416924886F1A8FABF40C491E。

## 2026-11-24 托盘状态颜色对齐

对照 HoverPanelView.vue、quotaPeriod.ts 和 styles.css，采用托盘的绿 #19c37d / 黄 #e9b949 / 红 #ff5a54 / 灰 #8fa19b。余量阈值改为 <30 / <60 / ≥60；周期按托盘四舍五入后判色，总体按原始均值判色。恢复时间分别使用 5 小时周期 1/3 小时、周周期 24/72 小时、月周期 3/7 天；订阅到期 3/7 天。实际采集时间超过 1 小时标红，集成已有过期/异常条件也标红，缺失为灰色。账号异常、缓存与推送中断保留文字提示。

- 模板测试 11/11 通过，增加阈值边界、逐周期时间、订阅到期、时钟跨阈值、采集过期、传输状态和账号状态覆盖。
- 原版组件浏览器检查包含实际 CSS 颜色验证、横竖屏及窄屏/矮横屏、纯文本安全、零额外网络请求。
- 仓库与文档导入 JSON 同步，当前 SHA-256：E40334203B5C50E73F7C4B4B610983D001CAD5C533ECAF92EC5084D1A9DD52AD。
- 未修改后端计算/投递或现用 PhoneBox，未重新打包安装程序；任务 6.2 仍待真实 Agent 联调。

## 2026-11-24 账号名称与托盘排序修正

根因是初版仅发送账号摘要别名，并按不透明 ID 排序；页面无法从旧负载恢复原账号名称。按用户要求增加 displayName 和 priorityResetAt：名称移除控制字符、最多 160 字符；优先恢复时间镜像托盘的月周期优先、最高优先级窗口与 nextResetAt 兜底规则，仅用于排序，不代替周期恢复时间。模板按订阅到期、优先恢复时间、名称 zh-CN localeCompare 排序，缺失时间最后，重排后按账号 ID 保留当前选中项。

隐私边界相应调整：允许展示账号名称，其中可能含邮箱；仍不发送独立邮箱字段、实例原名、凭据或其他原始标识，日志不包含展示负载。设置提示、接入文档与规格已同步。旧版发送端仍可显示备用别名；需要更新 TokenBall、立即推送并导入新模板才能获得完整效果。

- Rust 测试 51/51 通过，覆盖名称限长、控制字符过滤、排序时间选择、独立周期时间与其余敏感字段哨兵。
- 模板测试 13/13 通过；从真实 HoverPanelView.vue 提取排序函数进行对照，覆盖中文名称、订阅/恢复优先级、缺失时间、旧版别名、恶意名称纯文本与入站数据不变。
- 原版 HTML 组件沙箱浏览器检查通过：显示研发主账号、运营账号、备用账号等合成名称，打乱输入仍按托盘规则排序，修改订阅优先级后重排且保留当前账号；横竖屏布局、状态颜色、零额外网络请求检查通过。
- npm run build 与 test:quota-period（11/11）通过。仓库与文档 JSON SHA-256：5735D6AE4E9A54FC4A46DBD0E1BB58AC636B03B29219B3BD6DFFE44F44912D5B。
- 未读取或推送真实账号，未修改现用 PhoneBox 或重新打包安装程序；任务 6.2 和 USB/Wi-Fi 真机验收边界保持不变。
