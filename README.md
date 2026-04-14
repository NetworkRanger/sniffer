# Sniffer

一款跨平台的网络连接监控工具，能够实时展示系统中每个网络连接的**上传/下载速度**，并尝试将连接关联到**所属进程**和**访问域名**，帮助你清晰了解流量去向。

## 功能特性

- **连接级网速监控**：实时统计每个网络连接的上传和下载速率
- **进程关联**：通过系统 socket 信息匹配连接到对应进程，显示进程名称与可执行路径
- **域名还原**：深度解析数据包，尝试提取连接访问的域名
  - TLS ClientHello 中的 SNI
  - QUIC Initial Packet 解密后的 CRYPTO 帧
  - HTTP/1.1 Host、HTTP/2 Headers
  - DNS 查询报文
- **桌面 GUI**：基于 Tauri + Vue 3 的实时界面，直观展示连接列表、速率、进程图标
- **PCAP 导出**：可选将原始流量实时写入 `capture.pcap`，便于在 Wireshark 中进一步分析
- **命令行支持**：提供独立的 CLI 和示例程序，方便集成或二次开发

## 技术栈

| 层级 | 技术 |
|------|------|
| 底层抓包 | Rust、`pcap`、`pnet`、`pnet_datalink` |
| 协议解析 | `httparse`、`tls-parser`、`dns-parser`、`hpack_codec`、`ring` |
| 进程信息 | `netstat2`、`sysinfo` |
| 异步运行时 | `tokio`（GUI 侧） |
| 桌面框架 | Tauri v2 |
| 前端 | Vue 3（Composition API）+ TypeScript + Vite |
| 国际化 | `vue-i18n`（内置 `zh-CN`） |
| 日志 | `tracing` + `tracing-subscriber` |

## 快速开始

### 环境要求

- [Rust](https://rustup.rs/) 稳定版工具链
- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)（运行 GUI 需要）
- 系统安装 `libpcap`（Linux / macOS）
- **管理员 / root 权限**（抓包通常需要高权限运行）

### 构建 CLI

```bash
# 构建整个 Workspace
cargo build --verbose

# 运行命令行版本
cargo run -p sniffer

# 运行示例程序
cargo run --example sniffer1 -p sniffer
```

### 构建 GUI

```bash
cd sniffer-gui

# 安装前端依赖
pnpm install

# 开发模式（同时启动 Vite + Tauri）
pnpm tauri dev

# 生产构建
pnpm tauri build
```

前端常用脚本：
- `pnpm dev`：单独启动 Vite 开发服务器
- `pnpm build`：TypeScript 检查 + Vite 生产构建

## 项目结构

```
.
├── Cargo.toml                 # Workspace 根配置
├── sniffer-cli/               # Rust 库 + CLI 可执行文件
│   ├── src/
│   │   ├── lib.rs             # 库入口
│   │   ├── main.rs            # CLI 入口
│   │   ├── packet.rs          # 数据包解析、连接统计
│   │   ├── networking/types/  # 协议解析器（TLS、QUIC、H2C、HPACK、DNS）
│   │   └── utils/             # 工具函数、全局 Registry 缓存
│   └── examples/              # 示例程序
├── sniffer-gui/               # Tauri 桌面应用
│   ├── src/                   # Vue 3 前端源码
│   ├── src-tauri/             # Tauri Rust 后端
│   │   ├── src/
│   │   │   ├── capture.rs     # 抓包引擎
│   │   │   ├── aggregator.rs  # 数据聚合与速率计算
│   │   │   ├── process_connection.rs  # 系统 socket / 进程读取
│   │   │   ├── cache.rs       # 进程图标提取与缓存
│   │   │   └── pcap_writer.rs # PCAP 实时写入
│   │   └── Cargo.toml
│   └── package.json
└── .github/workflows/rust.yml # CI 配置
```

## 安全与权限

1. **抓包需要高权限**：`pcap` 的混杂模式通常要求以 root / sudo / 管理员身份运行。在 macOS 上可能需要在「安全性与隐私」中额外授权。
2. **进程信息敏感**：程序会读取系统所有网络连接和进程信息，图标缓存会写入用户目录 `~/.sniffer/`。
3. **PCAP 文件**：运行 GUI 时会在工作目录生成 `capture.pcap`，可能包含真实网络流量，**请勿将其提交到版本控制**（已列入 `.gitignore`）。

## 开发说明

- 源码注释、文档字符串、日志输出、前端文本均以**中文**为主。
- `sniffer-cli` 使用 Rust Edition 2024，`sniffer-gui/src-tauri` 使用 Edition 2021。
- 日志时间格式固定为 `yyyy-MM-dd HH:mm:ss.SSS`，时区为 `+8:00`（北京时间）。
- CLI/库侧大量使用 `.unwrap()` 和 `panic::catch_unwind` 来防止单包解析错误导致程序崩溃。
- 当前源码中暂无单元测试或集成测试，CI 仅执行 `cargo build` 和 `cargo test`。

## 许可证

[LICENSE](./LICENSE)
