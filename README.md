<div align="center">

# VibeAround

**Code with AI agents from anywhere — terminal, browser, phone, or chat.**

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

VibeAround is a unified runtime for AI coding agents. Start a session with Claude Code on your laptop, pick it up on Feishu from your phone, switch to Codex mid-conversation, and come back to the terminal — all on the same session.

It connects real agents (Claude Code, Gemini CLI, Codex, OpenCode) to every surface you use: desktop, browser, Telegram, Feishu, Discord, and WeChat. Not a wrapper — a runtime with full streaming, tool use, and thinking display.

## Screenshots

| Desktop | Mobile |
|---------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround web dashboard on desktop" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround web dashboard on mobile" /> |

## Key features

- **Session handover** — hand off a coding session from Claude Code CLI to any IM channel and continue on your phone *(currently Claude Code only; more agents coming)*
- **Agent switching** — `/switch claude`, `/switch codex`, `/switch gemini` mid-conversation from any channel
- **Web dashboard** — terminals, tmux, and agent chat at `localhost:12358`
- **IM channels** — Telegram, Feishu, Discord, WeChat — each a standalone plugin
- **Desktop app** — onboarding, service monitoring, workspace management, system tray
- **Multi-workspace** — manage project folders, set defaults, switch contexts
- **Tunnel access** — expose your dashboard via Cloudflare Tunnel, Ngrok, or Localtunnel

## Supported agents

All agents communicate via [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) over stdio. npm-based agents are auto-installed on first use.

| Agent | ACP | Session Handover |
|---|---|---|
| **Claude Code** | Working | Supported |
| **Gemini CLI** | Working | Coming soon |
| **Codex** | Working | Coming soon |
| **OpenCode** | Working | Coming soon |

## Channel plugins

Each channel is a standalone Node.js plugin built with [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk).

| Channel | Auth | Streaming edits | Status |
|---|---|---|---|
| **Telegram** | Bot token | Yes | Working |
| **Feishu / Lark** | App credentials | Yes (cards) | Working |
| **Discord** | Bot token | Yes | Working |
| **WeChat** | QR code login | No | Working |
| **WhatsApp** | Pairing code | No | Blocked ([upstream](https://github.com/WhiskeySockets/Baileys/issues/2422)) |

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

Hand off your Claude Code session to any connected IM channel:

```
you (Claude Code) > /vibearound handover
Claude Code       > Session ready. Send this in your IM chat:
                    /pickup claude abc123-session-id
```

Paste the `/pickup` command in Feishu, Telegram, Discord, or WeChat — continue the conversation right there. Works great for reviewing code on mobile.

> Session handover currently supports **Claude Code** only. Support for Gemini CLI, Codex, and OpenCode is in progress.

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
              │  │Agent Factory│  │   ← spawns Claude/Gemini/Codex/OpenCode
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

VibeAround is actively evolving and usable for daily work. Pull requests and feature requests are not being accepted at this time.

## License

[MIT](LICENSE)
