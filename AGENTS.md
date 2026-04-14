# Sniffer 项目指南

> 本文件供 AI 编码助手阅读，帮助你快速理解项目结构、构建方式和开发约定。

## 项目概述

Sniffer 是一款跨平台的网络流量嗅探与分析工具，由两部分组成：

- **sniffer-cli**：基于 Rust 的底层抓包库和命令行工具，使用 `pcap`/`pnet` 捕获并解析网络数据包，支持 TCP、UDP、HTTP、HTTPS、QUIC、H2C、DNS 等协议的深度解析。
- **sniffer-gui**：基于 **Tauri v2 + Vue 3 + TypeScript** 的桌面 GUI 应用，实时展示系统网络连接、进程归属、上传/下载速度，并支持进程图标显示和 PCAP 文件导出。

## 技术栈

- **后端**：Rust（Edition 2024 / 2021）
  - 抓包：`pcap`、`pnet`、`pnet_datalink`
  - 异步运行时：`tokio`（GUI 侧）
  - 日志：`tracing` + `tracing-subscriber`
  - 协议解析：`httparse`、`tls-parser`、`dns-parser`、`hpack_codec`
  - 加密：`aes`、`ctr`、`ecb`、`hmac-sha256`、`ring`
  - 进程/系统信息：`netstat2`、`sysinfo`
  - 跨平台图标提取：`winapi`（Windows）、`sips`（macOS）、桌面文件/图标主题（Linux）
- **前端**：Vue 3（Composition API）+ TypeScript + Vite
  - 国际化：`vue-i18n`（目前仅内置 `zh-CN`）
  - UI 组件：自定义组件（`ConnectionsTable`、`MenuBar`、`StatusBar`）
- **桌面框架**：Tauri v2

## 项目结构

```
.
├── Cargo.toml                 # Workspace 根配置
├── sniffer-cli/               # Rust 库 + CLI 可执行文件
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # 库入口，导出 packet、networking、utils、config
│   │   ├── main.rs            # CLI 入口，调用 cli_run()
│   │   ├── packet.rs          # 数据包解析、连接统计、Registry 读写
│   │   ├── config.rs          # 全局配置（频率、域名开关）
│   │   ├── networking/
│   │   │   └── types/         # 协议解析子模块
│   │   │       ├── mod.rs
│   │   │       ├── tls.rs     # TLS ClientHello / SNI 解析
│   │   │       ├── quic.rs    # QUIC Initial Packet 解密与域名提取
│   │   │       ├── h2c.rs     # HTTP/2 帧解析
│   │   │       ├── hpack.rs   # HPACK 解码
│   │   │       └── packet.rs  # 字节流读取辅助工具
│   │   └── utils/
│   │       ├── mod.rs         # get_mac_by_name 等工具函数
│   │       └── registry.rs    # 全局类型安全 KV 缓存（带过期机制）
│   └── examples/              # 示例程序（sniffer1/2/3.rs）
├── sniffer-gui/               # Tauri 桌面应用
│   ├── package.json           # pnpm 脚本与前端依赖
│   ├── src/                   # Vue 前端源码
│   │   ├── main.ts
│   │   ├── App.vue
│   │   ├── i18n.ts
│   │   ├── types/connection.ts
│   │   ├── locales/zh-CN.json
│   │   └── components/
│   │       ├── ConnectionsTable.vue
│   │       ├── MenuBar.vue
│   │       └── StatusBar.vue
│   └── src-tauri/             # Tauri Rust 后端
│       ├── Cargo.toml         # 依赖 sniffer-cli（path = ../../sniffer-cli）
│       ├── tauri.conf.json    # Tauri 应用配置
│       ├── capabilities/default.json
│       └── src/
│           ├── main.rs        # 程序入口
│           ├── lib.rs         # Tauri 命令注册与状态初始化
│           ├── models.rs      # AppState、Connection、NetworkStats 定义
│           ├── capture.rs     # CaptureEngine：抓包线程
│           ├── aggregator.rs  # Aggregator：数据聚合、速率计算、进程关联
│           ├── process_connection.rs  # 读取系统 socket 与进程信息
│           ├── pcap_writer.rs # 实时写入 capture.pcap
│           └── cache.rs       # 进程图标提取与缓存（内存 + 文件）
└── .github/workflows/rust.yml # CI：build + test
```

## 构建与运行

### 环境要求

- Rust toolchain（稳定版）
- Node.js + pnpm（用于 GUI 前端）
- 系统依赖：`libpcap`（Linux/macOS 需安装）
- **权限说明**：抓包通常需要管理员/root 权限运行。

### CLI 构建与运行

```bash
# 构建整个 workspace
cargo build --verbose

# 运行 CLI
cargo run -p sniffer

# 运行示例
cargo run --example sniffer1 -p sniffer
```

### GUI 构建与运行

```bash
cd sniffer-gui

# 安装前端依赖
pnpm install

# 开发模式（同时启动 Vite 和 Tauri）
pnpm tauri dev

# 生产构建
pnpm tauri build
```

前端脚本（定义在 `sniffer-gui/package.json`）：
- `pnpm dev`：单独启动 Vite 开发服务器
- `pnpm build`：TypeScript 检查 + Vite 生产构建
- `pnpm tauri`：调用 Tauri CLI

## 测试

当前项目源码中**没有编写单元测试或集成测试**。CI 流程中仅执行 `cargo test --verbose`，目前无实际测试用例。若新增测试，建议放在各 crate 的 `tests/` 目录或模块内的 `#[cfg(test)]` 中。

## 代码组织与关键模块

### sniffer-cli（核心库）

- **`packet.rs`**：最核心模块。负责 Ethernet → IPv4 → TCP/UDP 的逐层解析；维护 `Connection` 结构体；通过 `Registry` 缓存连接状态；统计上下行速率并输出日志。
- **`networking/types/`**：各协议专用解析器。
  - `tls.rs`：提取 TLS SNI（域名）。
  - `quic.rs`：基于 QUIC Initial Packet 的固定 salt，使用 HKDF/AES-CTR 解密并提取 CRYPTO 帧中的 server_name。
  - `h2c.rs` + `hpack.rs`：解析 HTTP/2 Headers 帧。
  - `packet.rs`：提供基于 `Cursor<Vec<u8>>` 的字节读取工具。
- **`utils/registry.rs`**：全局 `Registry`，基于 `std::sync::OnceLock` + `RwLock<HashMap>` 实现，支持按 key 设置/获取任意 `Send + Sync` 类型，并带默认 60 秒过期机制。

### sniffer-gui（桌面应用）

- **`lib.rs`**：注册 Tauri 命令（`get_network_stats`、`get_connections`、`stop_capture`），初始化 `AppState` 和日志订阅器，后台启动抓包线程。
- **`capture.rs`**：`CaptureEngine` 在独立线程中循环调用 `pcap::Capture::next_packet()`，将原始包转发给 `pcap_writer`，将解析后的 `PacketInfo` 发送给聚合器。
- **`aggregator.rs`**：`Aggregator::start()` 使用 `tokio::select!` 同时处理：
  1. 接收抓包数据并更新连接表；
  2. 每 50ms 刷新系统进程连接信息；
  3. 每秒计算一次全局速率统计，保留最近 300 秒历史。
- **`process_connection.rs`**：通过 `netstat2` 读取系统所有 TCP/UDP socket，结合 `sysinfo` 查询进程名、可执行路径、启动时间，并尝试获取进程图标。
- **`cache.rs`**：跨平台进程图标提取。
  - Windows：`ExtractIconW` + GDI 转 PNG。
  - macOS：查找 `.app/Contents/Resources/*.icns`，调用系统 `sips` 转 PNG。
  - Linux：遍历 `/usr/share/icons`、`/usr/share/pixmaps`、`.desktop` 文件查找图标。
  - 缓存策略：以 exe 路径 MD5 为 key，同时缓存在内存 `Mutex<HashMap>` 和用户目录 `~/.sniffer/*.png`。
- **`pcap_writer.rs`**：使用 `pcap-file` crate 将抓到的原始包实时写入项目根目录的 `capture.pcap`。

## 开发约定与代码风格

- **语言**：源码注释、文档字符串、日志输出、前端国际化文本均以**中文**为主。
- **Rust 版本**：`sniffer-cli` 使用 Edition 2024，`sniffer-gui/src-tauri` 使用 Edition 2021。
- **日志格式**：统一使用 `tracing`，时间格式固定为 `yyyy-MM-dd HH:mm:ss.SSS`，时区固定为 `+8:00`（北京时间）。
- **错误处理**：
  - CLI/库侧大量使用 `.unwrap()` 和 `panic::catch_unwind` 来防止单包解析错误导致整个程序崩溃。
  - GUI 侧 Tauri 命令返回 `Result<T, String>`，将错误信息传递给前端。
- **并发模型**：
  - CLI 使用 `std::thread` + `std::sync::mpsc::sync_channel`。
  - GUI 使用 `std::thread` 跑阻塞式 pcap 循环，通过 `tokio::sync::mpsc` 与异步聚合器通信；状态使用 `Arc<AppState>` 配合 `tokio::sync::RwLock` 共享。
- **前端风格**：Vue 3 `<script setup lang="ts">`，组合式 API，类型定义放在 `src/types/connection.ts`。

## CI / CD

`.github/workflows/rust.yml`：
- 触发条件：`push` / `pull_request` 到 `main` 分支
- 运行环境：`ubuntu-latest`
- 执行步骤：
  1. `cargo build --verbose`
  2. `cargo test --verbose`

> 注意：由于 Tauri 的构建需要完整前端产物和系统图形库依赖，当前 CI 仅构建 Rust workspace，未包含 Tauri 桌面应用的打包流程。

## 安全与权限注意事项

1. **抓包需要高权限**：`pcap` 的混杂模式通常要求以 root / sudo / 管理员身份运行。在 macOS 上可能需要在“安全性与隐私”中授权。
2. **CSP 设置**：`tauri.conf.json` 中 `csp` 当前为 `null`，生产发布前建议根据实际前端资源策略进行配置。
3. **进程信息敏感**：`process_connection.rs` 会读取系统所有网络连接和进程信息，图标缓存写入用户目录 `~/.sniffer/`。
4. **PCAP 文件**：运行 GUI 时会在工作目录生成 `capture.pcap`，可能包含未加密的真实网络流量，注意勿提交到版本控制（已列入 `.gitignore`）。

## 给 AI 助手的快速提示

- 修改协议解析逻辑时，重点看 `sniffer-cli/src/packet.rs` 和 `sniffer-cli/src/networking/types/`。
- 修改 GUI 数据流时，重点看 `sniffer-gui/src-tauri/src/capture.rs`、`aggregator.rs`、`process_connection.rs` 三者的协作关系。
- 新增前端页面或组件时，参考 `App.vue` 和现有三个 Vue 组件的组织方式。
- 添加国际化文本时，编辑 `sniffer-gui/src/locales/zh-CN.json`，并在 `src/types/connection.ts` 中补充类型（如有需要）。
