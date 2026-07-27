# 实例接入说明

这个页面用于快速完成实例配置。按下面步骤准备参数、填写表单、测试连接后，TokenBall 就能把不同额度来源统一展示到总览、悬浮球和悬停面板里。

## 接入流程

| 步骤 | 操作 | 结果 |
| --- | --- | --- |
| 1 | 在管理窗口点击对应 Provider 下的“新增实例” | 进入实例配置页 |
| 2 | 选择 Provider，填写显示名称、地址和凭证 | 生成一条本地实例配置 |
| 3 | 点击“保存” | 凭证写入本机配置存储，敏感字段不会回显 |
| 4 | 点击“测试” | 验证 TokenBall 能读取上游账号和额度 |
| 5 | 回到总览点击刷新 | 同步账号、额度窗口、剩余量和恢复时间 |

所有 Management Key、Access Key、Secret Key 和 Cookie 都只建议保存在可信本机环境。不要把本地数据库、日志、截图或备份文件提交到代码仓库。

## 参数速查

| Provider | 适合场景 | 必填参数 | 默认地址 |
| --- | --- | --- | --- |
| CLIProxyAPI | 已用 CLIProxyAPI 管理多个 Codex 账号 | 服务地址、管理 Key | `http://127.0.0.1:8317` |
| 火山引擎官方渠道 | 使用火山 OpenAPI 查询 Coding Plan / Agent Plan | Access Key ID、Secret Access Key | `https://open.volcengineapi.com` |
| 火山引擎页面渠道 | OpenAPI 暂时无法覆盖，需要用控制台登录态查询 | 控制台 Cookie | `https://console.volcengine.com/api/top` |
| 千问 Token Plan | 查询千问个人版 Token Plan 5 小时和每周窗口 | 控制台 Cookie | `https://platform-home.qianwenai.com` |

## CLIProxyAPI

适用于已经把多个 Codex 账号接入 CLIProxyAPI 的用户。TokenBall 通过 Management API 读取账号列表、账号状态、额度窗口和请求统计。

### 获取参数

| 参数 | 从哪里获取 | 填写建议 |
| --- | --- | --- |
| 服务地址 | CLIProxyAPI Management API 的监听地址 | 本机常见为 `http://127.0.0.1:8317`，远程实例填写实际内网或域名地址 |
| 管理 Key | CLIProxyAPI 的 `remote-management.secret-key` 或 `MANAGEMENT_PASSWORD` | 这是管理接口密钥，不是普通模型 API Key |

### 配置步骤

1. 在左侧 `CLIProxyAPI` 分组点击“新增实例”。
2. `服务地址` 填写 Management API 地址。
3. `管理 Key` 填写 CLIProxyAPI 管理密钥。
4. 点击“保存”，再点击“测试”。
5. 测试成功后回到总览刷新，确认账号出现在额度列表中。

### 常见问题

| 现象 | 处理方式 |
| --- | --- |
| 测试失败或 401 | 确认填写的是 Management Key，不是账号 API Key |
| 连接超时 | 确认 CLIProxyAPI 已启动，地址和端口能从本机访问 |
| 没有账号数据 | 先在 CLIProxyAPI 侧确认账号已接入并有额度统计 |

## 火山引擎

火山引擎支持官方渠道和页面渠道。官方渠道优先使用 OpenAPI，适合长期稳定接入；页面渠道使用控制台登录态 Cookie，适合临时补足 OpenAPI 未覆盖的数据。

### 官方渠道

通过火山 OpenAPI 查询套餐和用量。

| 参数 | 从哪里获取 | 填写建议 |
| --- | --- | --- |
| OpenAPI Host | TokenBall 默认提供 | 一般保持 `https://open.volcengineapi.com` |
| Access Key ID | 火山引擎控制台的访问控制 / AccessKey 管理 | 使用具有查询用量权限的 AK |
| Secret Access Key | 与 Access Key ID 对应的密钥 | 保存后不会回显，留空保存会沿用已保存值 |
| Region | 火山接口区域 | 默认 `cn-beijing` |
| Service | 火山服务名 | 默认 `ark` |
| Coding ProjectName | Coding Plan 查询参数 | 不确定时先保留 `default` |
| Coding SeatIDs | 高级席位查询参数 | 可留空；多个 SeatID 用逗号分隔 |

配置时选择 `火山引擎`，渠道类型选“官方渠道”，按表格填写后保存并测试。需要同时查看 Coding Plan 和 Agent Plan 时，保留对应勾选项即可。

### 页面渠道

页面渠道通过已登录控制台的请求 Cookie 查询 Coding Plan 用量。

| 参数 | 从哪里获取 | 填写建议 |
| --- | --- | --- |
| 控制台 API Host | TokenBall 默认提供 | 一般保持 `https://console.volcengine.com/api/top` |
| Coding ProjectName | 控制台当前项目 | 不确定时先保留 `default` |
| 控制台 Cookie | 浏览器里 `console.volcengine.com` 的已登录请求 | 可以粘贴完整 curl，也可以只粘贴 `Cookie` 请求头 |

获取 Cookie 的通用方法：

1. 在浏览器打开并登录火山引擎控制台。
2. 打开开发者工具，进入 Network 面板。
3. 刷新控制台页面，筛选 `console.volcengine.com` 请求。
4. 复制请求里的 `Cookie` 请求头，或复制该请求的完整 curl。
5. 粘贴到 TokenBall 的“控制台 Cookie”字段，离开输入框后会自动提取 Cookie。

## 千问 Token Plan

千问当前通过控制台登录态 Cookie 查询个人版 Token Plan 的主套餐和加油包用量，展示 5 小时窗口和每周窗口。

### 获取参数

| 参数 | 从哪里获取 | 填写建议 |
| --- | --- | --- |
| ProductCode | TokenBall 默认提供 | 一般保持 `token-plan` |
| 控制台网关 | TokenBall 默认提供 | 一般保持 `https://platform-home.qianwenai.com` |
| 控制台 Cookie | 浏览器里千问控制台已登录请求 | 可以粘贴完整 curl，也可以只粘贴 `Cookie` 请求头 |

获取 Cookie 的通用方法：

1. 在浏览器打开并登录千问控制台。
2. 打开开发者工具，进入 Network 面板。
3. 刷新 Token Plan 或个人额度相关页面。
4. 筛选 `platform.qianwenai.com` 或 `platform-home.qianwenai.com` 请求。
5. 复制请求里的 `Cookie` 请求头，或复制完整 curl。
6. 粘贴到 TokenBall 的“控制台 Cookie”字段，保存并测试。

千问接口字段和 `secToken` 获取细节见 [`/qianwen-token-plan-api`](/qianwen-token-plan-api)。

## 配置后的验证

| 检查项 | 正常表现 |
| --- | --- |
| 实例测试 | 页面提示“连接测试成功” |
| 总览刷新 | 实例下出现账号或套餐记录 |
| 额度窗口 | 能看到 5 小时、每周、每月或上游返回的窗口 |
| 悬浮球 | 开始展示总剩余额度、账号数量或连接状态 |
| 悬停面板 | 能看到账号所属实例、剩余量和恢复时间 |

## 维护建议

- Cookie 类实例失效时，重新从浏览器复制登录态请求并保存。
- Access Key 和 Management Key 轮换后，编辑实例重新填写密钥并测试。
- 导出配置文件会包含敏感凭证，只保存到可信位置。
- 多个团队、多个主账号或多个 CLIProxyAPI 地址建议拆成多个实例，方便单独停用、测试和排查。
