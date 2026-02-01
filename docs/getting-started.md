# Getting Started with 0-openclaw

This guide will help you set up 0-openclaw, the proof-carrying AI assistant.

## Prerequisites

- Rust 1.70+ (for building from source)
- Or Docker (for containerized deployment)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/0-protocol/0-openclaw
cd 0-openclaw

# Build
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```

### Using Docker

```bash
cd docker
cp .env.example .env
# Edit .env with your tokens
docker-compose up -d
```

## Quick Start

### 1. Initialize

```bash
zero-openclaw init
```

This creates:
- `~/.0-openclaw/config.json` - Configuration file
- `~/.0-openclaw/keypair` - Ed25519 keypair for signing
- `~/.0-openclaw/workspace/` - Skill workspace

### 2. Configure a Channel

Edit `~/.0-openclaw/config.json`:

```json
{
  "gateway": {
    "port": 18789,
    "bind": "127.0.0.1",
    "keypair_path": "~/.0-openclaw/keypair"
  },
  "channels": [
    {
      "type": "telegram",
      "enabled": true,
      "token": "YOUR_BOT_TOKEN",
      "allowlist": ["your_telegram_user_id"]
    }
  ],
  "skills": [
    "graphs/skills/echo.0"
  ]
}
```

### 3. Start the Gateway

```bash
zero-openclaw gateway
```

### 4. Test

Send a message to your bot. You should receive a response with a proof-carrying action.

## Verify Installation

```bash
# Check status
zero-openclaw status

# Run diagnostics
zero-openclaw doctor
```

## Next Steps

- [Configuration Guide](configuration.md)
- [Channel Setup](channels/)
- [Creating Skills](skills/creating-skills.md)

## Troubleshooting

### Gateway won't start

1. Check config: `zero-openclaw config validate`
2. Check port availability: `lsof -i :18789`
3. Check logs: `zero-openclaw gateway -v`

### Channel not connecting

1. Verify token is correct
2. Check allowlist includes your user ID
3. Enable verbose logging: `zero-openclaw gateway -v`

### Tests failing

```bash
cargo test -- --nocapture
```

## Getting Help

- [GitHub Issues](https://github.com/0-protocol/0-openclaw/issues)
- [Architecture Documentation](../ARCHITECTURE.md)
