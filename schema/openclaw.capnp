@0xc8d2d2f1a5e3b4c6;

# 0-openclaw Cap'n Proto Schema
#
# This schema defines the binary format for 0-openclaw data structures.
# Used for efficient serialization and network transmission.

# ============================================================================
# CORE MESSAGE TYPES
# ============================================================================

# Incoming message from any channel
struct Message {
  id @0 :Data;              # SHA-256 content hash (32 bytes)
  channelId @1 :Text;       # Channel identifier (e.g., "telegram")
  senderId @2 :Text;        # Sender's ID within the channel
  content @3 :Text;         # Message content
  timestamp @4 :UInt64;     # Unix timestamp in milliseconds
  metadata @5 :Data;        # JSON-encoded channel-specific metadata
}

# Outgoing message to a channel
struct OutgoingMessage {
  channelId @0 :Text;       # Target channel
  recipientId @1 :Text;     # Recipient's ID within the channel
  content @2 :Text;         # Message content
  replyTo @3 :Data;         # Optional: hash of message being replied to
}

# ============================================================================
# PROOF-CARRYING ACTIONS
# ============================================================================

# The core innovation: actions with cryptographic proof
struct ProofCarryingAction {
  action @0 :Action;              # The action to perform
  sessionHash @1 :Data;           # Hash of the session context (32 bytes)
  inputHash @2 :Data;             # Hash of the triggering input (32 bytes)
  executionTrace @3 :List(Data);  # Hashes of evaluated graph nodes
  confidence @4 :Float32;         # Confidence score (0.0 - 1.0)
  signature @5 :Data;             # Ed25519 signature (64 bytes)
  timestamp @6 :UInt64;           # Unix timestamp in milliseconds
}

# Actions the assistant can take
struct Action {
  union {
    sendMessage @0 :SendMessageAction;
    executeSkill @1 :ExecuteSkillAction;
    updateSession @2 :UpdateSessionAction;
    noOp @3 :NoOpAction;
  }
}

struct SendMessageAction {
  channelId @0 :Text;
  recipientId @1 :Text;
  content @2 :Text;
}

struct ExecuteSkillAction {
  skillHash @0 :Data;       # Content hash of the skill graph
  inputs @1 :Data;          # JSON-encoded inputs
}

struct UpdateSessionAction {
  sessionId @0 :Data;       # Session hash
  updates @1 :Data;         # JSON-encoded state updates
}

struct NoOpAction {
  reason @0 :Text;          # Why no action was taken
}

# ============================================================================
# SESSION MANAGEMENT
# ============================================================================

struct Session {
  id @0 :Data;              # Session hash
  channelId @1 :Text;       # Associated channel
  userId @2 :Text;          # Associated user
  state @3 :Data;           # Serialized session state
  history @4 :List(Data);   # Hashes of past actions
  trustScore @5 :Float32;   # Accumulated trust (0.0 - 1.0)
  createdAt @6 :UInt64;     # Creation timestamp
  lastActivity @7 :UInt64;  # Last activity timestamp
}

# ============================================================================
# SKILL DEFINITIONS
# ============================================================================

struct Skill {
  hash @0 :Data;            # Content hash of the skill graph
  name @1 :Text;            # Human-readable name
  description @2 :Text;     # Skill description
  version @3 :Text;         # Version string
  graphData @4 :Data;       # Serialized RuntimeGraph
  permissions @5 :List(Text); # Required permissions
  verified @6 :Bool;        # Whether skill has been verified
}

struct SkillMetadata {
  name @0 :Text;
  description @1 :Text;
  version @2 :Text;
  author @3 :Text;
  permissions @4 :List(Text);
  inputs @5 :List(SkillInput);
  outputs @6 :List(SkillOutput);
}

struct SkillInput {
  name @0 :Text;
  description @1 :Text;
  tensorType @2 :Text;
  required @3 :Bool;
}

struct SkillOutput {
  name @0 :Text;
  description @1 :Text;
  tensorType @2 :Text;
}

# ============================================================================
# VERIFICATION
# ============================================================================

struct VerificationResult {
  valid @0 :Bool;
  signatureValid @1 :Bool;
  traceValid @2 :Bool;
  confidenceValid @3 :Bool;
  errors @4 :List(Text);
  warnings @5 :List(Text);
}

# ============================================================================
# GATEWAY PROTOCOL
# ============================================================================

struct GatewayRequest {
  union {
    processMessage @0 :Message;
    getSession @1 :Data;            # Session hash
    listSessions @2 :Void;
    installSkill @3 :Skill;
    listSkills @4 :Void;
    verify @5 :ProofCarryingAction;
  }
}

struct GatewayResponse {
  union {
    action @0 :ProofCarryingAction;
    session @1 :Session;
    sessions @2 :List(Session);
    skillHash @3 :Data;
    skills @4 :List(Skill);
    verification @5 :VerificationResult;
    error @6 :Text;
  }
}
