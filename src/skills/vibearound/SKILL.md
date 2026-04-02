---
name: vibearound
description: Hand over your current coding session to an IM channel so you can continue the conversation on your phone or another device. Use when the user says "/vibearound handover", "hand over to feishu", "continue on my phone", "send this to discord", or similar session transfer requests.
---

# VibeAround Session Handover

Hand over the current Claude Code session to an IM channel via the VibeAround orchestrator, so the user can continue the conversation on another device.

## When to Use

- User says `/vibearound handover <channel>`
- User asks to "hand over", "transfer", or "continue" the session on an IM platform
- User mentions continuing on their phone, another device, or a specific chat app

## Prerequisites

The VibeAround MCP server must be connected (server name: `vibearound`). If not available, tell the user to start the VibeAround desktop app.

## Handover Steps

### 1. Resolve the session ID

```bash
cat ~/.claude/sessions/$PPID.json
```

Extract the `sessionId` field. If the file doesn't exist, inform the user.

### 2. Determine the target channel

Use the channel name the user mentioned (e.g. "feishu", "telegram", "discord", "weixin", "web"). If ambiguous, ask.

### 3. Call prepare_handover

```
Tool: prepare_handover
Server: vibearound
Arguments:
  target_channel: "<channel>"
  session_id: "<sessionId>"
  cwd: "<current working directory>"
  agent_kind: "claude"
```

If the tool says the workspace is not registered, ask the user for confirmation, then call `register_workspace` with the `cwd`, and retry.

### 4. Present the result

Show the `/pickup` command returned by the tool. The user sends it in their IM chat to resume the session there.

## Error Handling

- **MCP server not available**: Start the VibeAround desktop app.
- **Workspace not registered**: Offer to register it (needs user confirmation).
- **Channel not recognized**: Ask the user which channel they mean.
- **Session ID not found**: Session metadata file may not exist.
