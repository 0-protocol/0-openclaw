# Configuration Guide

0-openclaw uses a JSON configuration file, typically at `~/.0-openclaw/config.json`.

## Configuration Structure

```json
{
  "gateway": {
    "port": 18789,
    "bind": "127.0.0.1",
    "keypair_path": "~/.0-openclaw/keypair"
  },
  "channels": [],
  "skills": [],
  "agent": {
    "model": "anthropic/claude-opus-4-5",
    "workspace": "~/.0-openclaw/workspace"
  }
}
```

## Gateway Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `port` | number | 18789 | WebSocket server port |
| `bind` | string | "127.0.0.1" | Address to bind to |
| `keypair_path` | string | "~/.0-openclaw/keypair" | Path to Ed25519 keypair |

## Channel Configuration

Each channel has a common structure:

```json
{
  "type": "telegram",
  "enabled": true,
  "token": "BOT_TOKEN",
  "allowlist": ["user_id_1", "user_id_2"],
  "dm_policy": "allowlist",
  "group_policy": "mention"
}
```

| Key | Type | Description |
|-----|------|-------------|
| `type` | string | Channel type: telegram, discord, slack |
| `enabled` | boolean | Enable/disable this channel |
| `token` | string | Bot token |
| `allowlist` | array | User IDs allowed to interact |
| `dm_policy` | string | DM handling: "open", "allowlist", "pairing" |
| `group_policy` | string | Group handling: "mention", "always", "disabled" |

### Telegram-Specific

```json
{
  "type": "telegram",
  "token": "123456:ABC...",
  "webhook_url": "https://...",  // Optional
  "webhook_secret": "..."        // Optional
}
```

### Discord-Specific

```json
{
  "type": "discord",
  "token": "...",
  "application_id": 123456789,
  "guild_allowlist": [123, 456]  // Optional, restrict to specific servers
}
```

### Slack-Specific

```json
{
  "type": "slack",
  "bot_token": "xoxb-...",
  "app_token": "xapp-...",  // For Socket Mode
  "signing_secret": "..."
}
```

## Skills Configuration

List of skill graph files to load:

```json
{
  "skills": [
    "graphs/skills/echo.0",
    "graphs/skills/search.0",
    "~/.0-openclaw/workspace/skills/custom.0"
  ]
}
```

## Environment Variables

Environment variables override config file values:

| Variable | Config Key |
|----------|------------|
| `TELEGRAM_BOT_TOKEN` | channels[type=telegram].token |
| `DISCORD_BOT_TOKEN` | channels[type=discord].token |
| `SLACK_BOT_TOKEN` | channels[type=slack].bot_token |
| `SLACK_APP_TOKEN` | channels[type=slack].app_token |
| `GATEWAY_PORT` | gateway.port |

## Validation

Validate your configuration:

```bash
zero-openclaw config validate
```

## Example Configurations

### Minimal (Telegram only)

```json
{
  "gateway": {
    "port": 18789
  },
  "channels": [
    {
      "type": "telegram",
      "enabled": true,
      "token": "YOUR_TOKEN",
      "allowlist": ["YOUR_USER_ID"]
    }
  ],
  "skills": ["graphs/skills/echo.0"]
}
```

### Multi-Channel

```json
{
  "gateway": {
    "port": 18789,
    "bind": "0.0.0.0"
  },
  "channels": [
    {
      "type": "telegram",
      "enabled": true,
      "token": "TELEGRAM_TOKEN",
      "allowlist": ["user1", "user2"]
    },
    {
      "type": "discord",
      "enabled": true,
      "token": "DISCORD_TOKEN",
      "allowlist": ["user3"]
    }
  ],
  "skills": [
    "graphs/skills/echo.0",
    "graphs/skills/search.0",
    "graphs/skills/browser.0"
  ]
}
```
