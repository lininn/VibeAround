---
inclusion: always
name: va-session
description: Resolve your current session ID for use with other VibeAround tools
---

# VibeAround Session ID

## How to Resolve

### Method 1: Via VibeAround env vars (preferred)

Check if `VIBEAROUND_CHANNEL_KIND` and `VIBEAROUND_CHAT_ID` env vars are set. If yes:

```
Tool: get_session_id
Server: vibearound
Arguments:
  channel_kind: "<$VIBEAROUND_CHANNEL_KIND>"
  chat_id: "<$VIBEAROUND_CHAT_ID>"
```

### Method 2: Fallback

If env vars are not set, omit session_id — the server will attempt auto-discovery.
