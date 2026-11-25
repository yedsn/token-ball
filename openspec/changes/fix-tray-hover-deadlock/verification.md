# 验证记录

## 实现与证据

- 延迟任务仅等待 300ms 后投递界面线程；`commit_pending_hover` 在锁内复制状态，锁外查询与显示，查询后及显示后检查 generation。
- `hide_tray_hover` 将外部关闭与状态取消一起安排到界面线程；`hide_current_hover` 在隐藏前检查监控任务归属，锁外执行隐藏，失败限速重试。
- 检查全部悬浮锁作用域，没有跨越显示器查询、窗口操作或异步等待。现有延迟、隐藏宽限及抑制常量保持不变。
- Tauri runtime 2.11.4 的 `send_user_message` 在界面线程直接执行消息，其他线程投递消息；因此界面线程内查询不需要等待自己处理队列。

## 自动化测试

- 新增 6 个测试，覆盖阻塞查询期间取消、四种取消操作、查询/投递/显示失败后恢复、旧任务与旧监控器归属、显示重入、隐藏失败重试。
- 查询交错测试用通道暂停查询，并用 `try_lock` 确认另一参与者能取得锁；等待上限 2–3 秒，避免死锁导致测试永久挂起。
- `cargo test --manifest-path src-tauri/Cargo.toml`：首次 56 通过、1 失败；失败为已有 PhoneBox `disabled_sender_and_latest_signal_coalescing`，固定等待 30ms 后错误状态未到达。托盘测试全部通过。
- 单独复跑上述 PhoneBox 测试通过。
- `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`：57/57 通过，文档测试通过。未修改 PhoneBox 模块。
- `git diff --check` 通过。

## Windows 打包与桌面验收

- 版本 0.1.30，执行 `npm.cmd run tauri:build`；前端类型检查、生产构建与 Rust release 编译通过，生成 `src-tauri/target/release/token-ball.exe`、`bundle/msi/TokenBall_0.1.30_x64_en-US.msi`、`bundle/nsis/TokenBall_0.1.30_x64-setup.exe`。命令最终退出 1：缺少 `TAURI_SIGNING_PRIVATE_KEY`，无法生成自动更新签名。
- 正在使用临时配置关闭 `createUpdaterArtifacts` 验证本地安装包构建，不修改正式发布配置，不生成可发布的更新签名。
- 原故障进程 PID 57712 仍为未响应。尝试结束进程以启动修复版，Windows 返回 `Access is denied`；未成功结束旧实例，未启动替换版本。单实例约束使本次桌面验收受阻。
- agent-desktop 能读取任务栏与屏幕，但打开 system-tray 超时，system-tray-overflow 返回 WINDOW_NOT_FOUND；尝试通过任务栏入口后仍无法读取溢出区。桌面同时存在其他活动，尚未进行连续交互验收。
- 100 轮交错操作、两次后台刷新及修复版日志检查均未完成，不能由单元测试替代，不标记通过。
- 原故障没有调用栈，Windows 1460 超时与本次死锁路径的现场因果关系仍未最终确认。
