# VibeAround Session Handover

Hand over your current coding session to an IM channel (Feishu, Discord, WeChat, etc.) so you can continue the conversation on your phone or another device.

## Prerequisites

The VibeAround server must be running locally (default `http://127.0.0.1:12358/mcp`).

## Usage

When the user says `/vibearound handover <channel>`, follow these steps:

### 1. Get your session ID

Run this command to read your session ID:

```bash
cat ~/.claude/sessions/$PPID.json
```

Extract the `sessionId` field from the JSON output.

### 2. Call the handover tool

```
Tool: prepare_handover
Server: vibearound
Arguments:
  target_channel: "<channel>"      (feishu, telegram, discord, weixin, web)
  session_id: "<sessionId-from-step-1>"
  cwd: "<your-working-directory>"
  agent_kind: "claude"
```

If the workspace is not registered, the tool will tell you. Ask the user for confirmation, then call:

```
Tool: register_workspace
Server: vibearound
Arguments:
  cwd: "<your-working-directory>"
```

Then retry `prepare_handover`.

### 3. Show the result

After success, show the user the `/pickup` command returned by the tool. The user copies it and sends it in their IM chat.

## Supported Channels

| Channel | Value |
|---------|-------|
| Feishu / Lark | `feishu` |
| Telegram | `telegram` |
| Discord | `discord` |
| WeChat (OpenClaw) | `weixin` |
| Web Dashboard | `web` |
