# Spec Delta

## Purpose

提供随 TokenBall 分发、可由用户导入原版 PhoneBox 的额度页面，使手机和浏览器预览能够查看统一汇总、多账号分周期余量及恢复时间，并准确区分平台采集过期与传输中断，全程无需修改 PhoneBox 源码或在手机保存平台凭据。

## ADDED Requirements

### Requirement: Importable page without PhoneBox modifications

交付物 SHALL 包含可导入页面 JSON、自定义 HTML 源模板和接入说明。页面 MUST 遵循 PhoneBox `phonebox=pages`、`version=1` 的现有导入格式，使用普通 push 组件和内置 html 组件消费 `tokenball` key，不依赖新增 Agent 路由、Android 补丁或自动改写用户设置。

#### Scenario: User imports page on stock PhoneBox
- **WHEN** 用户在已验证兼容的原版 PhoneBox 中导入并保存页面，TokenBall 已成功推送
- **THEN** 浏览器预览及已连接手机可展示额度，不需要安装定制 Agent 或 APK，也不覆盖其他页面

### Requirement: Account overview and independent period cards

页面 SHALL 展示账号数量、可用账号数、平均余量及已知额度等效账号数，并在账号区展示脱敏别名、平台/实例/套餐、状态、5 小时/周/月剩余百分比和各自恢复时间、订阅到期及实际采集时间。缺失数据 MUST 显示 `--`；未知余量不得绘制为耗尽。存在遗漏账号时 MUST 明示遗漏数量，不能把详情列表宣称为全部账号。

#### Scenario: Partial period data
- **WHEN** 某账号仅有 5 小时及周额度，月额度缺失，周 resetAt 无效
- **THEN** 仅对应缺失值显示 `--`，其他周期正常展示，不借用恢复时间或订阅时间

#### Scenario: Large account list on portrait display
- **WHEN** 账号数量超出组件可视空间，或负载明确报告省略账号
- **THEN** 可见账号区能够滚动，布局不遮挡总览；省略数量在页面内明确可见

### Requirement: Honest cache and transport indicators

自定义 HTML SHALL 通过 `phonebox.on('push', ...)` 订阅更新，并用本地计时器更新倒计时和每个账号的 staleAt 判断。传输在线状态 MUST 与实际采集新鲜度分离；key 不存在、TTL 清除、心跳超过 180 秒或版本未知时给出对应说明，不伪造新的额度或采集时间。恢复倒计时到达 0 MUST 显示等待刷新，不自行设为满额。

#### Scenario: Connected sender with stale provider data
- **WHEN** 新心跳到达，但一个账号的采集时间过期，其他账号正常
- **THEN** 页面显示该账号为缓存/过期，不能因新投递把所有账号标为刚更新

#### Scenario: No new data while page remains visible
- **WHEN** 页面持续显示，账号跨过 staleAt，或已有心跳超过 180 秒
- **THEN** 无需收到新推送即可更新过期/连接中断提示

#### Scenario: Value expired or protocol unsupported
- **WHEN** tokenball key 被 TTL 移除或 schema 不受支持
- **THEN** 页面显示未收到数据/等待推送或版本不兼容，不显示假 0% 或静默解析为正常额度

### Requirement: Self-contained and inert display template

模板 MUST 不加载外部脚本、字体或图片，不直接访问 TokenBall 或平台接口，不调用桌面快捷指令；所有动态名称和文本 MUST 作为纯文本渲染。模板 SHALL 在横屏和竖屏中保持可读，源 HTML 与分发 JSON 内嵌 HTML 必须一致。

#### Scenario: Untrusted text in display data
- **WHEN** 套餐等展示字段包含 HTML 标签或脚本样式文本
- **THEN** 页面将其显示为普通文本，不执行脚本或发起额外网络请求

#### Scenario: Template regenerated for distribution
- **WHEN** 从 HTML 源模板生成页面 JSON 并运行一致性检查
- **THEN** 内嵌模板与源文件一致，导入文件可解析且组件 key/网格符合约定
