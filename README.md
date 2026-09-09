# RTC Inspector

RTC Inspector 是一个基于 Tauri 2、Rust、React、TypeScript 和 Apache ECharts 的轻量级 WebRTC 日志分析桌面工具。所有日志均在本机解析，不会上传到远程服务。

当前版本支持 macOS，并保留 Windows 构建能力。

## 已实现功能

- 导入未压缩或 gzip 压缩的 `webrtc-internals` dump
- 导入未压缩或 gzip 压缩的 `rtcstats` dump
- 自动识别日志格式并输出结构化解析错误
- 展示 PeerConnection 概览、事件、SDP、ICE 候选和 Stats 趋势
- 分析连接状态、信令状态、ICE 状态、媒体方向、编解码器、候选类型、选中候选对、RTT 和可用上行带宽
- 当日志没有直接记录 ICE gathering 状态时，根据候选和选中候选对显示 `complete（推断）`
- 一次选择多份日志并归入同一个 Session
- 向已有 Session 继续添加数据源
- 创建和切换多个 Session
- 在两个 Session 之间对比概览、事件、SDP、ICE 和 Stats
- Stats 对比以各 Session 起点对齐，展示发送码率、接收码率、RTT 和估算可用上行

## Session 规则

一次“新建 Session”操作中选择的一份或多份文件属于同一个 Session。同一通话的 `webrtc_internals_dump.gz` 和 `rtcstats_dump.gz` 可以同时选择，也可以先导入一份，再通过“添加数据源”补充另一份。

不同时间执行“新建 Session”会创建独立 Session。创建两个或更多 Session 后，可以在顶部切换到“对比”模式并选择 Baseline 与 Target。

同一个 Session 内，具有相同 PeerConnection ID 的内容会被聚合。重复 SDP、ICE 候选、候选对、事件和 Stats 数据会去重，同时保留数据源列表。

## 分析视图

| 视图 | 内容 |
| --- | --- |
| 概览 | 文件格式、数据源、持续时间、页面、浏览器、连接状态、媒体和网络摘要 |
| 事件 | PeerConnection API 调用和状态变化时间线 |
| SDP | Local/Remote Offer、Answer 和 SDP 原文 |
| ICE | 本地/远端候选、候选类型、选中路径、RTT 和可用带宽 |
| Stats | 发送/接收码率、丢包、帧率、RTT 和可用上行趋势 |

## 技术架构

```text
系统文件选择器
      |
      v
Tauri Command
      |
      v
Rust dump-reader
      |
      +-- adapter-rtcstats
      +-- adapter-internals
      |
      v
统一 RTC Domain Model
      |
      v
Session 聚合与去重
      |
      v
React 工作台 + ECharts
```

Rust 负责文件读取、gzip 解压、格式识别、解析和基础连接分析。React 前端负责 Session 工作区、数据源聚合、单 Session 查看和跨 Session 对比。

## 环境要求

- Node.js 20+
- npm 10+
- Rust stable（当前验证版本为 1.98）
- macOS：Xcode Command Line Tools
- Windows：Microsoft C++ Build Tools 和 WebView2 Runtime

## 安装依赖

```bash
npm install
```

需要代理时，可以在当前终端设置：

```bash
export https_proxy=http://127.0.0.1:7897
export http_proxy=http://127.0.0.1:7897
export all_proxy=socks5://127.0.0.1:7897
export NO_PROXY=localhost,127.0.0.1
```

## 开发运行

```bash
npm run dev
```

该命令会同时启动 Vite 开发服务和 Tauri 桌面窗口。Vite 默认监听 `http://localhost:1420/`，日志导入需要在 Tauri 桌面窗口中操作。

## 质量检查

```bash
npm run check
npm run test
npm run build
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## 构建安装包

macOS：

```bash
npm run tauri:build
```

产物位于：

```text
target/release/bundle/macos/RTC Inspector.app
target/release/bundle/dmg/RTC Inspector_<version>_aarch64.dmg
```

Windows 需要在 Windows 环境中执行相同构建命令，生成 NSIS 或 MSI 安装包。Tauri 使用系统 WebView2，不会将完整 Chromium 内核打入安装包。

## 项目结构

```text
apps/desktop/                     React、TypeScript、Vite 前端
apps/desktop/src-tauri/           Tauri 桌面入口、命令和应用资源
crates/rtc-domain/                统一 WebRTC 领域模型与连接分析
crates/dump-reader/               文件读取、gzip 解压和格式分派
crates/adapter-rtcstats/          rtcstats dump 解析器
crates/adapter-internals/         webrtc-internals dump 解析器
```

## 当前边界

- Session 当前保存在应用内存中，退出应用后不会恢复
- 当前一次只能选择两个 Session 进行对比
- Stats 按 Session 相对起点对齐，不执行跨设备绝对时钟校正
- 外部 Chrome CDP、内嵌 H5 SDK 和 WebRTC 重协商链路分析尚未接入
