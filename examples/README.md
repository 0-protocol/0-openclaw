# 0-openclaw Examples

This directory contains example configurations and skill graphs for 0-openclaw.

## Directory Structure

```
examples/
├── configs/           # Example configuration files
│   ├── minimal.json   # Minimal configuration
│   ├── telegram.json  # Telegram-only setup
│   └── full.json      # Full multi-channel setup
└── skills/            # Example skill graphs
    ├── echo.0         # Simple echo skill
    └── greeter.0      # Greeting skill
```

## Quick Start

1. Copy a configuration template:
   ```bash
   cp examples/configs/minimal.json ~/.0-openclaw/config.json
   ```

2. Edit with your credentials:
   ```bash
   $EDITOR ~/.0-openclaw/config.json
   ```

3. Start the gateway:
   ```bash
   zero-openclaw gateway
   ```

## Example Configuration

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
      "allowlist": ["your_user_id"]
    }
  ],
  "skills": [
    "graphs/skills/echo.0"
  ]
}
```

## Creating Skills

Skills are 0-lang graph files (`.0` extension). See the main documentation for
the graph format specification.

Basic skill structure:
```
Graph {
    name: "skill_name",
    version: 1,
    nodes: [...],
    outputs: ["output_node"],
}
```

## Verification

All examples can be verified:
```bash
# Verify a skill graph
zero-openclaw skill verify examples/skills/echo.0

# Verify a proof-carrying action
zero-openclaw verify action.pca
```

## More Information

- [README.md](../README.md) - Main documentation
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical architecture
- [0-lang](https://github.com/0-protocol/0-lang) - Graph language documentation
