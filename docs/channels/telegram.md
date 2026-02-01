# Telegram Channel Setup

## Prerequisites

1. Create a bot with [@BotFather](https://t.me/BotFather)
2. Get your bot token
3. Get your Telegram user ID (use [@userinfobot](https://t.me/userinfobot))

## Configuration

```json
{
  "channels": [
    {
      "type": "telegram",
      "enabled": true,
      "token": "123456789:ABCdefGHIjklMNOpqrsTUVwxyz",
      "allowlist": ["your_user_id"],
      "dm_policy": "allowlist",
      "group_policy": "mention"
    }
  ]
}
```

## Configuration Options

| Option | Values | Description |
|--------|--------|-------------|
| `dm_policy` | "open", "allowlist", "pairing" | How to handle DMs |
| `group_policy` | "mention", "always", "disabled" | How to handle group messages |

### DM Policies

- **open**: Accept DMs from anyone (not recommended)
- **allowlist**: Only respond to users in the allowlist
- **pairing**: Require pairing code for new users

### Group Policies

- **mention**: Only respond when @mentioned
- **always**: Respond to all messages
- **disabled**: Ignore group messages

## Webhook Mode (Optional)

For production, use webhooks instead of polling:

```json
{
  "type": "telegram",
  "token": "...",
  "webhook_url": "https://your-domain.com/webhook/telegram",
  "webhook_secret": "your-secret-string"
}
```

## Commands

The bot automatically supports:

- `/start` - Welcome message
- `/help` - Show available commands
- `/status` - Show bot status
- `/new` - Start new conversation

## Testing

```bash
# Start gateway
zero-openclaw gateway -v

# Send a message to your bot
# Check logs for processing
```

## Troubleshooting

### Bot not responding

1. Check token is correct
2. Verify your user ID is in allowlist
3. Check bot privacy settings with @BotFather

### Webhook not receiving

1. Verify webhook URL is accessible
2. Check SSL certificate is valid
3. Verify webhook secret matches

### Rate limiting

Telegram has rate limits. The bot handles these automatically with exponential backoff.
