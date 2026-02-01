# 0-openclaw Architecture

> Technical deep dive into the proof-carrying AI assistant architecture.

---

## System Overview

0-openclaw is built on three pillars:

1. **Graph-Native Processing** - All logic as verifiable DAGs
2. **Proof-Carrying Actions** - Every action is cryptographically traceable
3. **Confidence-Scored Permissions** - Probabilistic trust, not boolean gates

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              0-OPENCLAW                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                           0-GATEWAY                                 │ │
│  │                                                                     │ │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐          │ │
│  │  │   Session    │    │    Router    │    │    Skill     │          │ │
│  │  │   Manager    │    │    Graph     │    │   Registry   │          │ │
│  │  │              │    │              │    │              │          │ │
│  │  │ - Sessions   │    │ - Routing    │    │ - Skills     │          │ │
│  │  │ - Trust      │    │ - Matching   │    │ - Compose    │          │ │
│  │  │ - History    │    │ - Priority   │    │ - Verify     │          │ │
│  │  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘          │ │
│  │         │                   │                   │                   │ │
│  │         └───────────────────┼───────────────────┘                   │ │
│  │                             │                                       │ │
│  │                             ↓                                       │ │
│  │                   ┌──────────────────┐                              │ │
│  │                   │       0-VM       │                              │ │
│  │                   │  (Graph Engine)  │                              │ │
│  │                   │                  │                              │ │
│  │                   │ - Execute graphs │                              │ │
│  │                   │ - Generate trace │                              │ │
│  │                   │ - Compute conf.  │                              │ │
│  │                   └────────┬─────────┘                              │ │
│  │                            │                                        │ │
│  │                            ↓                                        │ │
│  │               ┌────────────────────────┐                            │ │
│  │               │   Proof Generator      │                            │ │
│  │               │                        │                            │ │
│  │               │ - Sign actions         │                            │ │
│  │               │ - Attach trace         │                            │ │
│  │               │ - Create PCA           │                            │ │
│  │               └────────────────────────┘                            │ │
│  │                                                                     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│           ↑              ↑              ↑              ↑                │
│      ┌────┴────┐    ┌────┴────┐    ┌────┴────┐    ┌────┴────┐          │
│      │Telegram │    │ Discord │    │  Slack  │    │ WebChat │          │
│      │Connector│    │Connector│    │Connector│    │Connector│          │
│      └─────────┘    └─────────┘    └─────────┘    └─────────┘          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Gateway

The Gateway is the central control plane that coordinates all operations.

```rust
pub struct Gateway {
    /// The 0-lang VM for graph execution
    vm: VM,
    
    /// Main routing graph
    router_graph: RuntimeGraph,
    
    /// Session manager
    sessions: Arc<RwLock<SessionManager>>,
    
    /// Registered channels
    channels: HashMap<String, Arc<dyn Channel>>,
    
    /// Skill registry
    skills: Arc<SkillRegistry>,
    
    /// Proof generator
    proof_generator: ProofGenerator,
    
    /// Gateway configuration
    config: GatewayConfig,
}
```

**Responsibilities:**
- Receive messages from channels
- Route messages through graphs
- Execute skills
- Generate proof-carrying actions
- Manage sessions

### 2. Session Manager

Manages conversation state with confidence-based trust.

```rust
pub struct Session {
    pub id: ContentHash,
    pub channel_id: String,
    pub user_id: String,
    pub state: SessionState,
    pub history: Vec<ContentHash>,  // Hashes of past actions
    pub trust_score: Confidence,    // Accumulated trust
    pub created_at: u64,
    pub last_activity: u64,
}
```

**Trust Evolution:**
- New users start with neutral confidence (0.5)
- Successful interactions increase trust
- Suspicious patterns decrease trust
- Trust decays over time without interaction

### 3. Router Graph

Content-addressed routing for deterministic message handling.

```
Graph {
    name: "main_router",
    nodes: [
        // Input parsing
        { id: "parse", type: External, uri: "input://message" },
        
        // Command detection
        { id: "is_command", type: Operation, 
          op: StartsWith, inputs: ["parse", "/"] },
        
        // Multi-path routing
        { id: "route", type: Route,
          input: "parse",
          routes: [
            { condition: "is_command", target: "command_handler" },
            { condition: "default", target: "conversation_handler" },
          ]
        },
    ],
}
```

**Properties:**
- Same input → Same route (deterministic)
- Route decisions are in the trace
- Routes are verifiable

### 4. Skill Registry

Manages verified skill graphs.

```rust
pub struct SkillRegistry {
    /// Installed skill graphs (content-addressed)
    skills: HashMap<ContentHash, SkillEntry>,
    
    /// Name to hash mapping
    name_index: HashMap<String, ContentHash>,
}

pub struct SkillEntry {
    pub hash: ContentHash,
    pub metadata: SkillMetadata,
    pub graph: RuntimeGraph,
    pub verified: bool,
}
```

**Skill Composition:**
```rust
let composer = SkillComposer::new();
composer.add_skill(search_skill);
composer.add_skill(summarize_skill);
composer.connect("search", "results", "summarize", "input");
let composed = composer.compose()?;
```

### 5. Proof Generator

Creates cryptographic proofs for every action.

```rust
pub struct ProofGenerator {
    signing_key: SigningKey,
}

impl ProofGenerator {
    pub fn generate(
        &self,
        action: Action,
        session_hash: ContentHash,
        input_hash: ContentHash,
        traces: Vec<ExecutionTrace>,
    ) -> Result<ProofCarryingAction, ProofError> {
        // Combine traces
        let execution_trace = self.combine_traces(traces);
        
        // Calculate confidence
        let confidence = self.calculate_confidence(&execution_trace);
        
        // Sign everything
        let signature = self.sign(&action, &session_hash, &input_hash, 
                                   &execution_trace, confidence);
        
        Ok(ProofCarryingAction {
            action,
            session_hash,
            input_hash,
            execution_trace,
            confidence,
            signature,
            timestamp: now(),
        })
    }
}
```

---

## Core Concepts

### Graph-Based Logic

All logic in 0-openclaw is expressed as directed acyclic graphs (DAGs).

```
┌───────────────────────────────────────────────────────────┐
│                    MESSAGE FLOW                            │
├───────────────────────────────────────────────────────────┤
│                                                            │
│    ┌─────────┐                                             │
│    │  Input  │                                             │
│    └────┬────┘                                             │
│         │                                                  │
│         ↓                                                  │
│    ┌─────────┐     ┌─────────┐                            │
│    │  Parse  │ ──→ │ Validate│                            │
│    └────┬────┘     └────┬────┘                            │
│         │               │                                  │
│         └───────┬───────┘                                  │
│                 │                                          │
│                 ↓                                          │
│           ┌─────────┐                                      │
│           │  Route  │                                      │
│           └────┬────┘                                      │
│                │                                           │
│       ┌────────┼────────┐                                  │
│       ↓        ↓        ↓                                  │
│  ┌────────┐ ┌────────┐ ┌────────┐                         │
│  │ Skill1 │ │ Skill2 │ │ Skill3 │                         │
│  └───┬────┘ └───┬────┘ └───┬────┘                         │
│      │          │          │                               │
│      └──────────┼──────────┘                               │
│                 ↓                                          │
│           ┌─────────┐                                      │
│           │ Output  │                                      │
│           └─────────┘                                      │
│                                                            │
└───────────────────────────────────────────────────────────┘
```

**Benefits:**
- Execution is deterministic
- Each node produces a hash
- The entire path is traceable

### Content-Addressed Routing

Routes are determined by content hashes, not runtime state.

```rust
// Traditional routing (non-deterministic)
fn route(msg: &str) -> Handler {
    if msg.starts_with("/help") { return help_handler; }
    if context.last_message.was_question() { return qa_handler; }
    default_handler
}

// 0-openclaw routing (deterministic)
Graph {
    nodes: [
        { id: hash("help_check"), type: Operation, 
          op: StartsWith, inputs: ["msg", "/help"] },
        { id: hash("qa_check"), type: Operation,
          op: IsQuestion, inputs: ["msg"] },
        { id: hash("router"), type: Route,
          routes: [
            { condition: hash("help_check"), target: hash("help_skill") },
            { condition: hash("qa_check"), target: hash("qa_skill") },
            { condition: "default", target: hash("default_skill") },
          ]
        },
    ],
}
```

**Guarantee:** Given the same input hash, routing produces the same output hash.

### Proof-Carrying Actions

Every action includes cryptographic proof of its origin.

```
┌─────────────────────────────────────────────────────────────┐
│                  PROOF-CARRYING ACTION                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  action: SendMessage {                                       │
│    channel: "telegram",                                      │
│    recipient: "user_123",                                    │
│    content: "Hello!"                                         │
│  }                                                           │
│                                                              │
│  session_hash: 0xABC123...                                   │
│  input_hash: 0xDEF456...                                     │
│                                                              │
│  execution_trace: [                                          │
│    0x111... (parse node)                                     │
│    0x222... (validate node)                                  │
│    0x333... (route node)                                     │
│    0x444... (skill node)                                     │
│    0x555... (output node)                                    │
│  ]                                                           │
│                                                              │
│  confidence: 0.95                                            │
│  timestamp: 1706886400000                                    │
│                                                              │
│  signature: ed25519(...)                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Verification:**
1. Check signature validity
2. Verify trace nodes exist in registered graphs
3. Confirm confidence meets threshold
4. Validate input hash matches the triggering message

### Confidence Scores

Permissions use probabilistic confidence instead of boolean decisions.

```
┌─────────────────────────────────────────────────────────────┐
│                  CONFIDENCE SCORING                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Factors Contributing to Confidence:                         │
│                                                              │
│  ┌─────────────────────┐                                     │
│  │ User Trust Score    │ ──→ 0.85 (established user)        │
│  └─────────────────────┘                                     │
│                                                              │
│  ┌─────────────────────┐                                     │
│  │ Action Risk Level   │ ──→ 0.70 (moderate risk action)    │
│  └─────────────────────┘                                     │
│                                                              │
│  ┌─────────────────────┐                                     │
│  │ Context Confidence  │ ──→ 0.90 (clear intent)            │
│  └─────────────────────┘                                     │
│                                                              │
│  Combined: geometric_mean(0.85, 0.70, 0.90) = 0.81          │
│                                                              │
│  Threshold: 0.80                                             │
│  Result: ✓ ALLOWED (0.81 >= 0.80)                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Skill Composition

Skills are modular graphs that can be combined.

```
┌─────────────────────────────────────────────────────────────┐
│                  SKILL COMPOSITION                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐        ┌─────────────────┐             │
│  │   Search Skill  │        │ Summarize Skill │             │
│  │                 │        │                 │             │
│  │  query → search │        │  text → summary │             │
│  │  search → result│        │  summary → out  │             │
│  └────────┬────────┘        └────────┬────────┘             │
│           │                          │                       │
│           │    ┌─────────────────┐   │                       │
│           └───→│   Composer      │←──┘                       │
│                │                 │                           │
│                │ connect:        │                           │
│                │ search.result   │                           │
│                │     ↓           │                           │
│                │ summarize.text  │                           │
│                └────────┬────────┘                           │
│                         │                                    │
│                         ↓                                    │
│              ┌─────────────────────┐                        │
│              │  Composed Skill     │                        │
│              │                     │                        │
│              │  query → search →   │                        │
│              │  result → summarize │                        │
│              │  → summary → out    │                        │
│              └─────────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Message Processing Flow

```
1. Channel receives message
   │
   ↓
2. Gateway creates IncomingMessage
   │
   ↓
3. Session Manager gets/creates session
   │
   ↓
4. Router Graph executes
   │  └── Produces route_trace
   │
   ↓
5. Selected Skill Graph executes
   │  └── Produces skill_trace
   │
   ↓
6. Proof Generator creates PCA
   │  └── Combines traces
   │  └── Calculates confidence
   │  └── Signs action
   │
   ↓
7. Action is executed
   │
   ↓
8. Session is updated
```

### Verification Flow

```
1. Receive PCA for verification
   │
   ↓
2. Verify signature
   │  └── Check Ed25519 signature
   │  └── Reject if invalid
   │
   ↓
3. Validate execution trace
   │  └── Each hash exists in registered graphs
   │  └── Order is valid (respects DAG)
   │
   ↓
4. Check confidence threshold
   │  └── confidence >= required_threshold
   │
   ↓
5. Verify input hash (optional)
   │  └── Hash matches original message
   │
   ↓
6. Return verification result
```

---

## File Structure

```
0-openclaw/
├── src/
│   ├── lib.rs              # Library exports
│   ├── types.rs            # Core types (ContentHash, Confidence, etc.)
│   ├── error.rs            # Error types
│   ├── gateway/
│   │   ├── mod.rs          # Gateway struct
│   │   ├── session.rs      # Session management
│   │   ├── router.rs       # Message routing
│   │   ├── proof.rs        # Proof generation
│   │   └── server.rs       # WebSocket server
│   ├── channels/
│   │   ├── mod.rs          # Channel trait
│   │   ├── telegram/       # Telegram connector
│   │   ├── discord/        # Discord connector
│   │   └── slack/          # Slack connector
│   ├── skills/
│   │   ├── mod.rs          # Skill registry
│   │   ├── composer.rs     # Skill composition
│   │   ├── verifier.rs     # Skill verification
│   │   └── builtin/        # Built-in skills
│   └── cli/
│       ├── mod.rs          # CLI main
│       └── commands/       # CLI commands
├── graphs/
│   ├── core/               # Core routing graphs
│   ├── channels/           # Channel-specific graphs
│   └── skills/             # Skill graphs
├── schema/
│   └── openclaw.capnp      # Cap'n Proto schema
└── examples/
    └── ...                 # Example configurations
```

---

## Security Model

### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    TRUST BOUNDARIES                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  UNTRUSTED                    VERIFIED                       │
│  ─────────                    ────────                       │
│                                                              │
│  ┌─────────────┐             ┌─────────────┐                │
│  │  External   │             │   Gateway   │                │
│  │  Messages   │ ──────────→ │   (0-VM)    │                │
│  └─────────────┘   verify    └─────────────┘                │
│                    before                                    │
│  ┌─────────────┐   execute   ┌─────────────┐                │
│  │  User       │             │   Skill     │                │
│  │  Skills     │ ──────────→ │   Registry  │                │
│  └─────────────┘   verify    └─────────────┘                │
│                    before                                    │
│  ┌─────────────┐   install   ┌─────────────┐                │
│  │  Channel    │             │   Proof     │                │
│  │  Events     │ ──────────→ │   Actions   │                │
│  └─────────────┘   sign      └─────────────┘                │
│                    after                                     │
│                    process                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Verification Points

1. **Skill Installation** - Skills are verified before installation
2. **Message Processing** - All paths produce verifiable traces
3. **Action Execution** - Actions are signed before execution
4. **External Calls** - External calls are logged in trace

---

## Performance Considerations

### Graph Caching

- Compiled graphs are cached by content hash
- Same graph = same cache entry
- Cache invalidation is automatic (content-addressed)

### Trace Optimization

- Traces only store hashes (32 bytes each)
- Full node data retrieved on verification
- Streaming traces for long executions

### Confidence Calculation

- Precomputed for common paths
- Cached per session
- Incremental updates on interaction

---

## Extension Points

### Custom Channels

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn processing_graph(&self) -> &RuntimeGraph;
    async fn receive(&self) -> Result<IncomingMessage, ChannelError>;
    async fn send(&self, msg: OutgoingMessage) -> Result<(), ChannelError>;
    fn evaluate_permission(&self, action: &Action, sender: &str) -> Confidence;
}
```

### Custom Skills

Create a `.0` graph file and register with the skill registry:

```rust
let skill_graph = RuntimeGraph::load_from_file("my_skill.0")?;
registry.install_graph("my_skill", skill_graph, false)?;
```

### Custom Confidence Evaluators

```rust
pub trait ConfidenceEvaluator: Send + Sync {
    fn evaluate(&self, context: &EvaluationContext) -> Confidence;
}
```

---

## References

- [0-lang Specification](https://github.com/0-protocol/0-lang)
- [0-protocol Architecture](https://github.com/0-protocol)
- [OpenClaw Documentation](https://openclaw.ai)

---

<div align="center">

**∅**

*Architecture for verifiable AI assistants.*

</div>
