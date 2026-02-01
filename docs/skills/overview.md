# Skills Overview

Skills are the building blocks of 0-openclaw's capabilities. Each skill is a verified graph that processes inputs and produces outputs.

## What is a Skill?

A skill is a directed acyclic graph (DAG) that:

1. Receives inputs (messages, data)
2. Processes through graph nodes
3. Produces outputs (responses, actions)
4. Generates execution traces for verification

## Built-in Skills

| Skill | File | Description |
|-------|------|-------------|
| Echo | `echo.0` | Simple echo - returns input |
| Search | `search.0` | Web search integration |
| Browser | `browser.0` | Web page interaction |
| Calendar | `calendar.0` | Calendar management |

## Skill Structure

```
graphs/skills/
├── echo.0          # Simple echo skill
├── search.0        # Search skill
├── browser.0       # Browser skill
├── calendar.0      # Calendar skill
└── custom/         # Your custom skills
```

## Loading Skills

Skills are loaded from the configuration:

```json
{
  "skills": [
    "graphs/skills/echo.0",
    "graphs/skills/search.0",
    "~/.0-openclaw/workspace/skills/my-skill.0"
  ]
}
```

## Skill Properties

Every skill has:

- **Content Hash**: Unique identifier based on graph content
- **Metadata**: Name, description, version, permissions
- **Inputs**: What the skill accepts
- **Outputs**: What the skill produces
- **Permissions**: Required access (network, files, etc.)

## Verification

All skills are verified before execution:

```bash
# Verify a skill
zero-openclaw skill verify graphs/skills/echo.0
```

Verification checks:
- No infinite loops
- Valid node references
- Permission requirements
- Type consistency

## Skill Composition

Skills can be composed into workflows:

```bash
zero-openclaw skill compose search.0 summarize.0 --output search-and-summarize.0
```

See [Creating Skills](creating-skills.md) for more details.
