<div align="center">

<img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/documents/v0.1/banner.webp" width="100%" alt="VibeAround — Unified runtime for AI coding agents" />

# VibeAround

**Unified runtime for AI coding agents — terminal, browser, phone, or chat.**

[English](README.md) | [简体中文](README_CN.md) | [Wiki](https://github.com/jazzenchen/VibeAround/wiki)

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.82+-000?style=flat-square&logo=rust&logoColor=fff" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-2.10-24C8DB?style=flat-square&logo=tauri&logoColor=fff" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=000" alt="React" />
  <img src="https://img.shields.io/badge/ACP-Rust_SDK-000?style=flat-square" alt="ACP" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT" />
</p>

</div>

VibeAround is a unified runtime for AI coding agents. It connects real agents (Claude Code, Gemini CLI, Codex CLI, Cursor CLI, Kiro CLI, Qwen Code, OpenCode) to every surface you use: desktop, browser, Telegram, Feishu, Discord, Slack, and WeChat. Not a wrapper — a runtime with full streaming, tool use, and thinking display.

Start a task with Claude Code on your Mac, hand it over to Telegram on your phone, continue the conversation with full context, and hand it back when you're at your desk.

## Key features

- **Web terminal** — full PTY-based terminal in the browser with tmux integration, run shell sessions alongside agent chat
- **Session handover** — hand off a coding session from any agent to any IM channel and continue on your phone
- **Agent switching** — `/switch claude`, `/switch codex`, `/switch cursor` mid-conversation from any channel
- **Web dashboard** — terminals, tmux, and agent chat at `localhost:12358`
- **IM channels** — Telegram, Feishu, Discord, Slack, WeChat — each a standalone plugin
- **Desktop app** — onboarding with install progress, service monitoring, workspace management, system tray
- **Multi-workspace** — manage project folders, set defaults, switch contexts
- **Tunnel access** — expose your dashboard via Cloudflare Tunnel, Ngrok, or Localtunnel

## Supported agents

All agents communicate via [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) over stdio. npm-based agents are auto-installed on first use. CLI-based agents (Cursor, Kiro, Qwen, OpenCode) must be installed by the user.

| Agent | ACP | Session Handover |
|---|---|---|
| **Claude Code** | Working | Supported |
| **Gemini CLI** | Working | Supported |
| **Codex CLI** | Working | Supported |
| **Cursor CLI** | Working | Supported |
| **Kiro CLI** | Working | Supported |
| **Qwen Code** | Working | Supported |
| **OpenCode** | Working | Not supported |

## Channel plugins

Each channel is a standalone Node.js plugin built with [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk).

| Channel | Auth | DM | File/Image | Streaming | Slash Commands | Status |
|---|---|---|---|---|---|---|
| **Telegram** | Bot token | Yes | Yes | Yes | `/command` | Working |
| **Feishu / Lark** | App credentials | Yes | Yes | Yes (cards) | `/command` | Working |
| **Discord** | Bot token | Yes | Yes | Yes | `/command` | Working |
| **Slack** | Bot + App token | Yes | Yes | Yes | `/va`, `/vibearound` | Working |
| **WeChat** | QR code login | Yes | No | No | `/command` | Working |

## Commands

### System commands

| Command | Description |
|---|---|
| `/help` | Show available commands |
| `/new` | Reset session (new conversation) |
| `/switch <agent>` | Switch agent (claude, gemini, codex, cursor, kiro, qwen-code, opencode) |
| `/profile <name>` | Switch profile |
| `/close` | Close conversation |
| `/pickup <code>` | Resume a coding agent session |
| `/handover` | Export session to a coding agent CLI |

### Agent commands

| Command | Description |
|---|---|
| `/agent <command>` | Send a slash command to the agent (e.g. `/agent status`) |

### Slack-specific

In Slack, the `/` prefix is intercepted by the client. Use `/va` or `/vibearound` instead:

| Slack command | Equivalent |
|---|---|
| `/va help` | `/help` |
| `/va switch claude` | `/switch claude` |
| `/va agent status` | `/agent status` |
| `/va new` | `/new` |

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| **Rust** | 1.82+ | [rustup.rs](https://rustup.rs/) |
| **Node.js** | 20+ | [nodejs.org](https://nodejs.org/) |
| **Bun** | 1.1+ | [bun.sh](https://bun.sh/) |
| **npm** | 10+ | Included with Node.js |

macOS only. Xcode Command Line Tools required:

```bash
xcode-select --install
```

## Quick start

```bash
cd src
bun install
bun run prebuild
bun run dev
```

1. Desktop app opens with onboarding wizard
2. Choose agents, configure channels, set up tunnel
3. Web dashboard at `http://127.0.0.1:12358`
4. Start coding through terminals, chat, or IM channels

## Session handover

Hand off your coding session to any connected IM channel — works with Claude Code, Gemini CLI, Codex CLI, Cursor CLI, Kiro CLI, and Qwen Code:

```
you (terminal)  > /handover
Agent           > Handover ready. Copied to clipboard:
                  /pickup V5RX
                  Paste it in any IM chat connected to VibeAround.
                  The code expires in 2 minutes.
```

Paste the `/pickup` command in Telegram, Feishu, Discord, Slack, or WeChat — continue the conversation with full context. When you're done, `/handover` again to return the session to your terminal.

## Architecture

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Desktop    │  │    Web      │  │  IM Channel │
│  (Tauri)    │  │  Dashboard  │  │  Plugins    │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
              ┌─────────┴─────────┐
              │   Rust Runtime    │
              │  ┌─────────────┐  │
              │  │  ACP Hub    │  │   ← routes prompts to agents
              │  │  (per-route │  │
              │  │   ACPPod)   │  │
              │  └──────┬──────┘  │
              │         │         │
              │  ┌──────┴──────┐  │
              │  │Agent Factory│  │   ← spawns Claude/Gemini/Codex/Cursor/Kiro/Qwen/OpenCode
              │  └─────────────┘  │
              │                   │
              │  ┌─────────────┐  │
              │  │ PTY Manager │  │   ← terminal sessions + tmux
              │  └─────────────┘  │
              └───────────────────┘
```

## Configuration

All config lives in `~/.vibearound/settings.json`:

```json
{
  "default_agent": "claude",
  "enabled_agents": ["claude", "gemini", "opencode", "codex", "cursor", "kiro", "qwen-code"],
  "workspaces": ["/path/to/your/project"],
  "channels": {
    "telegram": { "bot_token": "..." },
    "feishu": { "app_id": "...", "app_secret": "..." },
    "discord": { "bot_token": "..." },
    "slack": { "bot_token": "xoxb-...", "app_token": "xapp-..." }
  },
  "tunnel": {
    "provider": "cloudflare",
    "cloudflare": { "tunnel_token": "...", "hostname": "..." }
  }
}
```

## Plugin SDK

Build your own channel plugin:

```bash
npm install @vibearound/plugin-channel-sdk
```

See the [SDK README](https://github.com/jazzenchen/vibearound-plugin-channel-sdk) for the full guide.

## Documentation

- [Wiki Home](https://github.com/jazzenchen/VibeAround/wiki)
- [Setup Guide](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide)
- [Channel Plugins](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins)
- [Architecture](https://github.com/jazzenchen/VibeAround/wiki/Architecture)
- [Configuration](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model)
- [FAQ & Troubleshooting](https://github.com/jazzenchen/VibeAround/wiki/FAQ-and-Troubleshooting)

## Project status

VibeAround is actively evolving and usable for daily work.

## Roadmap

### More IM channels

| Channel | Status |
|---|---|
| LINE | In development |
| Microsoft Teams | In development |
| DingTalk | Planned |
| QQ | Planned |

### Workspace management

- Multi-project workspace switching and persistence
- Per-workspace agent and channel configuration
- Workspace-level session history and context

## License

[MIT](LICENSE)
