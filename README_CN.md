<div align="center">

# VibeAround

**随时随地用 AI 写代码 — 终端、浏览器、手机、聊天应用。**

[English](README.md) | [简体中文](README_CN.md) | [Wiki](https://github.com/jazzenchen/VibeAround/wiki)

<p>
  <img src="Logo.png" width="120" alt="VibeAround" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.82+-000?style=flat-square&logo=rust&logoColor=fff" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-2.10-24C8DB?style=flat-square&logo=tauri&logoColor=fff" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=000" alt="React" />
  <img src="https://img.shields.io/badge/ACP-Rust_SDK-000?style=flat-square" alt="ACP" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT" />
</p>

</div>

VibeAround 是 AI 编程代理的统一运行时。在电脑上用 Claude Code 开始一段编程会话，在飞书上用手机接着聊，中途切换到 Codex，再回到终端 — 全程同一个会话。

它将真正的编程代理（Claude Code、Gemini CLI、Codex、OpenCode）接入你日常使用的每个界面：桌面应用、浏览器、Telegram、飞书、Discord、微信。不是套壳 — 是一个完整的运行时，支持流式输出、工具调用和思考过程展示。

## 截图

| 桌面端 | 移动端 |
|---------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround 网页控制台" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround 移动端" /> |

## 核心功能

- **会话接力** — 将 Claude Code CLI 的编程会话一键移交到任意 IM 频道，手机上继续对话 *（目前仅支持 Claude Code，其他 agent 陆续支持中）*
- **Agent 切换** — 在任何频道中 `/switch claude`、`/switch codex`、`/switch gemini` 随时切换
- **网页控制台** — 终端、tmux、agent 对话，访问 `localhost:12358`
- **IM 频道** — Telegram、飞书、Discord、微信 — 每个都是独立插件
- **桌面应用** — 引导向导、服务监控、工作空间管理、系统托盘
- **多工作空间** — 管理项目目录、设置默认、切换上下文
- **隧道访问** — 通过 Cloudflare Tunnel、Ngrok 或 Localtunnel 远程访问

## 支持的 Agents

所有 agent 通过 [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) 经由 stdio 通信。基于 npm 的 agent 首次使用时自动安装。

| Agent | ACP | 会话接力 |
|---|---|---|
| **Claude Code** | 可用 | 已支持 |
| **Gemini CLI** | 可用 | 即将支持 |
| **Codex** | 可用 | 即将支持 |
| **OpenCode** | 可用 | 即将支持 |

## 频道插件

每个频道都是独立的 Node.js 插件，基于 [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk) 构建。

| 频道 | 认证方式 | 流式编辑 | 状态 |
|---|---|---|---|
| **Telegram** | Bot Token | 支持 | 可用 |
| **飞书 / Lark** | 应用凭证 | 支持（卡片） | 可用 |
| **Discord** | Bot Token | 支持 | 可用 |
| **微信** | 二维码登录 | 不支持 | 可用 |
| **WhatsApp** | 配对码 | 不支持 | 被[上游问题](https://github.com/WhiskeySockets/Baileys/issues/2422)阻塞 |

## 快速开始

```bash
cd src
bun install
bun run prebuild
bun run dev
```

1. 首次运行时桌面应用会打开引导向导
2. 选择 agents，配置频道和隧道
3. 网页控制台：`http://127.0.0.1:12358`
4. 通过终端、对话或 IM 频道开始编程

## 会话接力

将 Claude Code 的编程会话移交到任意已连接的 IM 频道：

```
你 (Claude Code) > /vibearound handover
Claude Code      > 会话已准备好。在 IM 中发送：
                   /pickup claude abc123-session-id
```

在飞书、Telegram、Discord 或微信中粘贴 `/pickup` 命令 — 直接继续对话。非常适合在手机上 review 代码。

> 会话接力目前仅支持 **Claude Code**。Gemini CLI、Codex 和 OpenCode 的支持正在开发中。

## 架构

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   桌面端    │  │  网页控制台  │  │  IM 频道    │
│  (Tauri)    │  │  Dashboard  │  │   插件      │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
              ┌─────────┴─────────┐
              │   Rust 运行时     │
              │  ┌─────────────┐  │
              │  │  ACP Hub    │  │   ← 将 prompt 路由到 agent
              │  │ (按路由分配  │  │
              │  │   ACPPod)   │  │
              │  └──────┬──────┘  │
              │         │         │
              │  ┌──────┴──────┐  │
              │  │ Agent 工厂  │  │   ← 启动 Claude/Gemini/Codex/OpenCode
              │  └─────────────┘  │
              │                   │
              │  ┌─────────────┐  │
              │  │ PTY 管理器  │  │   ← 终端会话 + tmux
              │  └─────────────┘  │
              └───────────────────┘
```

## 配置

所有配置位于 `~/.vibearound/settings.json`：

```json
{
  "default_agent": "claude",
  "enabled_agents": ["claude", "gemini", "opencode", "codex"],
  "workspaces": ["/path/to/your/project"],
  "channels": {
    "telegram": { "bot_token": "..." },
    "feishu": { "app_id": "...", "app_secret": "..." },
    "discord": { "bot_token": "..." }
  },
  "tunnel": {
    "provider": "cloudflare",
    "cloudflare": { "tunnel_token": "...", "hostname": "..." }
  }
}
```

## 插件 SDK

使用 SDK 构建自己的频道插件：

```bash
npm install @vibearound/plugin-channel-sdk
```

详见 [SDK README](https://github.com/jazzenchen/vibearound-plugin-channel-sdk)。

## 文档

- [Wiki 首页](https://github.com/jazzenchen/VibeAround/wiki)
- [安装指南](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide)
- [频道插件](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins)
- [架构](https://github.com/jazzenchen/VibeAround/wiki/Architecture)
- [配置模型](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model)
- [FAQ 和故障排除](https://github.com/jazzenchen/VibeAround/wiki/FAQ-and-Troubleshooting)

## 项目状态

VibeAround 正在积极迭代，当前版本已可用于日常工作。暂不接受 PR 和功能请求。

## 许可证

[MIT](LICENSE)
