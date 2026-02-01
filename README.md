# 0-openclaw

> **Every action carries proof. Every decision is verifiable.**

The first AI assistant where you don't have to trust the code—you verify it.

[![Built with 0-lang](https://img.shields.io/badge/Built_with-0--lang-black.svg)](https://github.com/0-protocol/0-lang)
[![Proof-Carrying](https://img.shields.io/badge/Actions-Proof--Carrying-blue.svg)](#proof-carrying-actions)
[![License](https://img.shields.io/badge/License-Apache_2.0-white.svg)](LICENSE)

---

## The Problem with Traditional AI Assistants

Traditional assistants like OpenClaw, while powerful, share a fundamental flaw:

| Issue | Reality |
|-------|---------|
| **Trust-based execution** | You hope the code does what it claims |
| **Opaque decisions** | Why did it send that message? |
| **Debug by prayer** | Logs are post-hoc, incomplete |
| **Security theater** | Allowlists are just strings |
| **Non-deterministic** | Same input might produce different results |

When you run a traditional assistant, you're trusting:
- The runtime doesn't have bugs
- The permissions are checked correctly
- The logs capture what actually happened
- No malicious code path was triggered

**You can't verify any of this.** You can only trust.

---

## 0-openclaw: Proof-Carrying AI Assistant

0-openclaw rebuilds the assistant paradigm from first principles using [0-lang](https://github.com/0-protocol/0-lang):

| Traditional | 0-openclaw |
|-------------|------------|
| Trust the code | **Verify the proof** |
| Boolean permissions | **Confidence-scored trust** |
| Text-based routing | **Content-addressed graphs** |
| Hope it's safe | **Cryptographic guarantees** |
| Debug via logs | **Replay via trace** |

### Core Innovations

1. **Proof-Carrying Actions** - Every action includes cryptographic proof of the decision path
2. **Content-Addressed Logic** - Same hash = same behavior, always
3. **Confidence Scores** - Probabilistic trust instead of boolean gates
4. **Composable Skill Graphs** - Verified, shareable, auditable skill modules

---

## How It Works

Every message through 0-openclaw produces a **Proof-Carrying Action**:

```
┌─────────────────────────────────────────────────────────────┐
│  Incoming Message                                           │
│       ↓                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ Parse Graph │ →  │ Route Graph │ →  │ Skill Graph │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│       ↓                   ↓                   ↓             │
│  [input_hash]      [route_trace]      [execution_trace]    │
│       └───────────────────┼───────────────────┘            │
│                           ↓                                 │
│              ┌─────────────────────────┐                   │
│              │   Proof-Carrying Action │                   │
│              │   - action: SendMessage │                   │
│              │   - proof: [hashes...]  │                   │
│              │   - confidence: 0.95    │                   │
│              │   - signature: ed25519  │                   │
│              └─────────────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

The proof includes:
- **Input Hash**: SHA-256 of the triggering message
- **Execution Trace**: Ordered list of graph nodes evaluated
- **Confidence Score**: Probabilistic trust level (0.0 - 1.0)
- **Signature**: Ed25519 signature over all fields

Anyone can verify that:
1. The action was triggered by this specific input
2. The decision followed this exact path through the graphs
3. The confidence meets the required threshold
4. The signature is valid

---

## Why 0-lang? A Real Comparison

### OpenClaw (TypeScript) - Trust Required

```typescript
// You must trust this code does what it says
async function handleMessage(msg: Message) {
  if (this.allowList.includes(msg.sender)) {  // String comparison
    const response = await this.agent.process(msg);  // Black box
    await this.channel.send(response);  // Hope it's right
  }
}
```

**Problems:**
- How do you know `allowList.includes()` wasn't bypassed?
- What happened inside `agent.process()`?
- Did `channel.send()` actually send what you think?

### 0-openclaw (0-lang) - Verify Instead

```
Graph {
  name: "message_handler",
  nodes: [
    // Permission check with confidence score
    { id: 0xABC..., type: Permission, 
      subject: "sender", 
      threshold: 0.8 
    },
    // Route based on content hash (deterministic)
    { id: 0xDEF..., type: Route, 
      input: "message",
      routes: [...]
    },
    // Execute skill with proof generation
    { id: 0x123..., type: SkillExecute, 
      skill: "responder",
      proof_required: true 
    },
  ],
  // Every execution produces verifiable trace
  proof_policy: ProofPolicy::Always,
}
```

**Guarantees:**
- Permission check happened (it's in the trace)
- Routing was deterministic (same hash = same route)
- Skill execution is recorded (proof attached)
- You can replay and verify the entire flow

---

## Features

### Multi-Channel Support

Connect to messaging platforms with verifiable message handling:

- **Telegram** - Bot API with command graphs
- **Discord** - Slash commands with permission proofs
- **Slack** - Events API with audit trails
- **WhatsApp** - Business API with end-to-end verification

### Composable Skills

Skills are verified graph modules that can be combined:

```
graphs/skills/
├── echo.0        # Simple echo (verified)
├── search.0      # Web search (verified)
├── browser.0     # Web automation (verified)
└── custom/       # Your verified skills
```

### Confidence-Based Permissions

Instead of boolean allow/deny, 0-openclaw uses confidence scores:

```rust
// Traditional: binary decision
if allowlist.contains(sender) { allow() } else { deny() }

// 0-openclaw: confidence-scored
let confidence = evaluate_trust(sender, action, context);
if confidence.meets_threshold(0.8) {
    execute_with_proof(action, confidence)
} else {
    request_verification(action, confidence)
}
```

---

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/0-protocol/0-openclaw
cd 0-openclaw

# Build
cargo build --release

# Initialize
./target/release/zero-openclaw init
```

### Configuration

Create `~/.0-openclaw/config.json`:

```json
{
  "gateway": {
    "port": 18789,
    "keypair_path": "~/.0-openclaw/keypair"
  },
  "channels": [
    {
      "type": "telegram",
      "token": "YOUR_BOT_TOKEN",
      "allowlist": ["your_user_id"]
    }
  ],
  "skills": [
    "graphs/skills/echo.0"
  ]
}
```

### Run

```bash
# Start the gateway
zero-openclaw gateway

# In another terminal, check status
zero-openclaw status

# Verify a proof-carrying action
zero-openclaw verify action.pca
```

---

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed technical documentation.

```
┌─────────────────────────────────────────────────────────────────┐
│                         0-OPENCLAW                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                      0-GATEWAY                            │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │   │
│  │  │  Session   │  │   Router   │  │   Skill    │          │   │
│  │  │  Manager   │  │   Graph    │  │  Registry  │          │   │
│  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘          │   │
│  │        └───────────────┼───────────────┘                  │   │
│  │                        ↓                                  │   │
│  │              ┌──────────────────┐                         │   │
│  │              │      0-VM        │                         │   │
│  │              │  (Graph Engine)  │                         │   │
│  │              └────────┬─────────┘                         │   │
│  │                       ↓                                   │   │
│  │          Proof-Carrying Actions                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│         ↑              ↑              ↑              ↑          │
│    ┌────┴────┐    ┌────┴────┐    ┌────┴────┐    ┌────┴────┐    │
│    │Telegram │    │ Discord │    │  Slack  │    │   ...   │    │
│    │Connector│    │Connector│    │Connector│    │         │    │
│    └─────────┘    └─────────┘    └─────────┘    └─────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Comparison with OpenClaw

| Feature | OpenClaw | 0-openclaw |
|---------|----------|------------|
| Language | TypeScript | 0-lang (Rust) |
| Execution Model | Imperative | Graph-based |
| Verification | Trust-based | Proof-carrying |
| Permissions | Boolean allowlists | Confidence scores |
| Debugging | Log analysis | Trace replay |
| Skill System | JavaScript modules | Verified graphs |
| Determinism | Non-deterministic | Content-addressed |

---

## Proof-Carrying Actions

The core innovation of 0-openclaw is the Proof-Carrying Action (PCA):

```rust
pub struct ProofCarryingAction {
    /// The action to perform
    pub action: Action,
    
    /// Hash of the session context
    pub session_hash: [u8; 32],
    
    /// Hash of the input that triggered this action
    pub input_hash: [u8; 32],
    
    /// Hashes of all graph nodes evaluated
    pub execution_trace: Vec<[u8; 32]>,
    
    /// Confidence score for this action (0.0 - 1.0)
    pub confidence: f32,
    
    /// Ed25519 signature over all fields
    pub signature: [u8; 64],
    
    /// Timestamp of action generation
    pub timestamp: u64,
}
```

### Verification

```bash
# Verify a PCA file
zero-openclaw verify action.pca

# Output:
# ✓ Signature valid
# ✓ Execution trace valid (12 nodes)
# ✓ Confidence: 0.95 (threshold: 0.80)
# ✓ Input hash matches message
# ✓ Session hash matches context
```

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

0-openclaw follows the 0-protocol contribution model:
- Proof-carrying code changes
- Verified before merge
- Content-addressed commits

---

## License

Apache 2.0 - See [LICENSE](LICENSE)

---

<div align="center">

**∅**

*Don't trust. Verify.*

[Documentation](ARCHITECTURE.md) · [0-lang](https://github.com/0-protocol/0-lang) · [0-protocol](https://github.com/0-protocol)

</div>
