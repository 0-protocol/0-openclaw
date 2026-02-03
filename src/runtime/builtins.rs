//! Built-in operations for the 0-lang runtime.
//!
//! These are the primitive operations that can be used in 0-lang graphs.
//! All complex logic should be built by composing these primitives.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use super::types::Value;
use crate::error::GatewayError;

/// A built-in operation.
#[async_trait]
pub trait BuiltinOp: Send + Sync {
    /// Execute the operation with given inputs and parameters.
    async fn execute(
        &self,
        inputs: Vec<Value>,
        params: &serde_json::Value,
    ) -> Result<Value, GatewayError>;
    
    /// Get the operation name.
    fn name(&self) -> &str;
    
    /// Get the operation description.
    fn description(&self) -> &str {
        ""
    }
}

/// Registry of built-in operations.
pub struct BuiltinRegistry {
    ops: HashMap<String, Arc<dyn BuiltinOp>>,
}

impl BuiltinRegistry {
    /// Create a new registry with all standard builtins.
    pub fn new() -> Self {
        let mut registry = Self {
            ops: HashMap::new(),
        };
        
        // Register all builtins
        registry.register(Arc::new(IdentityOp));
        registry.register(Arc::new(StartsWithOp));
        registry.register(Arc::new(EndsWithOp));
        registry.register(Arc::new(ContainsOp));
        registry.register(Arc::new(ExtractFirstWordOp));
        registry.register(Arc::new(ExtractParamsOp));
        registry.register(Arc::new(ConcatOp));
        registry.register(Arc::new(SplitOp));
        registry.register(Arc::new(TrimOp));
        registry.register(Arc::new(ToLowerOp));
        registry.register(Arc::new(ToUpperOp));
        registry.register(Arc::new(LengthOp));
        registry.register(Arc::new(GetFieldOp));
        registry.register(Arc::new(SetFieldOp));
        registry.register(Arc::new(MultiplyOp));
        registry.register(Arc::new(AddOp));
        registry.register(Arc::new(SubtractOp));
        registry.register(Arc::new(DivideOp));
        registry.register(Arc::new(EqualsOp));
        registry.register(Arc::new(NotEqualsOp));
        registry.register(Arc::new(GreaterThanOp));
        registry.register(Arc::new(LessThanOp));
        registry.register(Arc::new(AndOp));
        registry.register(Arc::new(OrOp));
        registry.register(Arc::new(NotOp));
        registry.register(Arc::new(IfOp));
        registry.register(Arc::new(HashOp));
        registry.register(Arc::new(SignOp));
        registry.register(Arc::new(VerifyOp));
        registry.register(Arc::new(TimestampOp));
        registry.register(Arc::new(ClassifyIntentOp));
        registry.register(Arc::new(LoadStateOp));
        registry.register(Arc::new(SaveStateOp));
        registry.register(Arc::new(CreateMapOp));
        registry.register(Arc::new(MergeMapOp));
        registry.register(Arc::new(ArrayPushOp));
        registry.register(Arc::new(ArrayGetOp));
        
        registry
    }
    
    /// Register a builtin operation.
    pub fn register(&mut self, op: Arc<dyn BuiltinOp>) {
        self.ops.insert(op.name().to_string(), op);
    }
    
    /// Get a builtin by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn BuiltinOp>> {
        self.ops.get(name)
    }
    
    /// List all builtin names.
    pub fn list(&self) -> Vec<&str> {
        self.ops.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get the number of builtins.
    pub fn len(&self) -> usize {
        self.ops.len()
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// String Operations
// ============================================================================

struct IdentityOp;

#[async_trait]
impl BuiltinOp for IdentityOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        Ok(inputs.into_iter().next().unwrap_or(Value::Null))
    }
    fn name(&self) -> &str { "Identity" }
    fn description(&self) -> &str { "Returns the input unchanged" }
}

struct StartsWithOp;

#[async_trait]
impl BuiltinOp for StartsWithOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
        Ok(Value::Bool(input.starts_with(prefix)))
    }
    fn name(&self) -> &str { "StartsWith" }
}

struct EndsWithOp;

#[async_trait]
impl BuiltinOp for EndsWithOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let suffix = params.get("suffix").and_then(|v| v.as_str()).unwrap_or("");
        Ok(Value::Bool(input.ends_with(suffix)))
    }
    fn name(&self) -> &str { "EndsWith" }
}

struct ContainsOp;

#[async_trait]
impl BuiltinOp for ContainsOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let pattern = params.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        Ok(Value::Bool(input.contains(pattern)))
    }
    fn name(&self) -> &str { "Contains" }
}

struct ExtractFirstWordOp;

#[async_trait]
impl BuiltinOp for ExtractFirstWordOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let first_word = input.split_whitespace().next().unwrap_or("");
        Ok(Value::String(first_word.to_string()))
    }
    fn name(&self) -> &str { "ExtractFirstWord" }
}

struct ExtractParamsOp;

#[async_trait]
impl BuiltinOp for ExtractParamsOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let words: Vec<&str> = input.split_whitespace().skip(1).collect();
        let params: Vec<Value> = words.iter().map(|w| Value::String(w.to_string())).collect();
        Ok(Value::Array(params))
    }
    fn name(&self) -> &str { "ExtractParams" }
}

struct ConcatOp;

#[async_trait]
impl BuiltinOp for ConcatOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let separator = params.get("separator").and_then(|v| v.as_str()).unwrap_or("");
        let strings: Vec<String> = inputs.iter()
            .filter_map(|v| v.as_string().map(|s| s.to_string()))
            .collect();
        Ok(Value::String(strings.join(separator)))
    }
    fn name(&self) -> &str { "Concat" }
}

struct SplitOp;

#[async_trait]
impl BuiltinOp for SplitOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let separator = params.get("separator").and_then(|v| v.as_str()).unwrap_or(" ");
        let parts: Vec<Value> = input.split(separator)
            .map(|s| Value::String(s.to_string()))
            .collect();
        Ok(Value::Array(parts))
    }
    fn name(&self) -> &str { "Split" }
}

struct TrimOp;

#[async_trait]
impl BuiltinOp for TrimOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        Ok(Value::String(input.trim().to_string()))
    }
    fn name(&self) -> &str { "Trim" }
}

struct ToLowerOp;

#[async_trait]
impl BuiltinOp for ToLowerOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        Ok(Value::String(input.to_lowercase()))
    }
    fn name(&self) -> &str { "ToLower" }
}

struct ToUpperOp;

#[async_trait]
impl BuiltinOp for ToUpperOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        Ok(Value::String(input.to_uppercase()))
    }
    fn name(&self) -> &str { "ToUpper" }
}

struct LengthOp;

#[async_trait]
impl BuiltinOp for LengthOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let len = match inputs.first() {
            Some(Value::String(s)) => s.len(),
            Some(Value::Array(a)) => a.len(),
            Some(Value::Map(m)) => m.len(),
            Some(Value::Bytes(b)) => b.len(),
            _ => 0,
        };
        Ok(Value::Int(len as i64))
    }
    fn name(&self) -> &str { "Length" }
}

// ============================================================================
// Map Operations
// ============================================================================

struct GetFieldOp;

#[async_trait]
impl BuiltinOp for GetFieldOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let field = params.get("field").and_then(|v| v.as_str()).unwrap_or("");
        match inputs.first() {
            Some(Value::Map(m)) => Ok(m.get(field).cloned().unwrap_or(Value::Null)),
            _ => Ok(Value::Null),
        }
    }
    fn name(&self) -> &str { "GetField" }
}

struct SetFieldOp;

#[async_trait]
impl BuiltinOp for SetFieldOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let field = params.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let mut map = match inputs.first() {
            Some(Value::Map(m)) => m.clone(),
            _ => HashMap::new(),
        };
        if let Some(value) = inputs.get(1) {
            map.insert(field.to_string(), value.clone());
        }
        Ok(Value::Map(map))
    }
    fn name(&self) -> &str { "SetField" }
}

struct CreateMapOp;

#[async_trait]
impl BuiltinOp for CreateMapOp {
    async fn execute(&self, _inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let mut map = HashMap::new();
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v));
            }
        }
        Ok(Value::Map(map))
    }
    fn name(&self) -> &str { "CreateMap" }
}

struct MergeMapOp;

#[async_trait]
impl BuiltinOp for MergeMapOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let mut result = HashMap::new();
        for input in inputs {
            if let Value::Map(m) = input {
                result.extend(m);
            }
        }
        Ok(Value::Map(result))
    }
    fn name(&self) -> &str { "MergeMap" }
}

// ============================================================================
// Array Operations
// ============================================================================

struct ArrayPushOp;

#[async_trait]
impl BuiltinOp for ArrayPushOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let mut array = match inputs.first() {
            Some(Value::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        if let Some(value) = inputs.get(1) {
            array.push(value.clone());
        }
        Ok(Value::Array(array))
    }
    fn name(&self) -> &str { "ArrayPush" }
}

struct ArrayGetOp;

#[async_trait]
impl BuiltinOp for ArrayGetOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let index = params.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        match inputs.first() {
            Some(Value::Array(a)) => Ok(a.get(index).cloned().unwrap_or(Value::Null)),
            _ => Ok(Value::Null),
        }
    }
    fn name(&self) -> &str { "ArrayGet" }
}

// ============================================================================
// Math Operations
// ============================================================================

struct MultiplyOp;

#[async_trait]
impl BuiltinOp for MultiplyOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let mut result = 1.0;
        for input in inputs {
            if let Some(v) = input.as_float() {
                result *= v;
            }
        }
        Ok(Value::Float(result))
    }
    fn name(&self) -> &str { "Multiply" }
}

struct AddOp;

#[async_trait]
impl BuiltinOp for AddOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let mut result = 0.0;
        for input in inputs {
            if let Some(v) = input.as_float() {
                result += v;
            }
        }
        Ok(Value::Float(result))
    }
    fn name(&self) -> &str { "Add" }
}

struct SubtractOp;

#[async_trait]
impl BuiltinOp for SubtractOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let second = inputs.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        Ok(Value::Float(first - second))
    }
    fn name(&self) -> &str { "Subtract" }
}

struct DivideOp;

#[async_trait]
impl BuiltinOp for DivideOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let second = inputs.get(1).and_then(|v| v.as_float()).unwrap_or(1.0);
        if second == 0.0 {
            return Err(GatewayError::ExecutionError("Division by zero".to_string()));
        }
        Ok(Value::Float(first / second))
    }
    fn name(&self) -> &str { "Divide" }
}

// ============================================================================
// Comparison Operations
// ============================================================================

struct EqualsOp;

#[async_trait]
impl BuiltinOp for EqualsOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().cloned().unwrap_or(Value::Null);
        let second = inputs.get(1).cloned().unwrap_or(Value::Null);
        Ok(Value::Bool(first == second))
    }
    fn name(&self) -> &str { "Equals" }
}

struct NotEqualsOp;

#[async_trait]
impl BuiltinOp for NotEqualsOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().cloned().unwrap_or(Value::Null);
        let second = inputs.get(1).cloned().unwrap_or(Value::Null);
        Ok(Value::Bool(first != second))
    }
    fn name(&self) -> &str { "NotEquals" }
}

struct GreaterThanOp;

#[async_trait]
impl BuiltinOp for GreaterThanOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let second = inputs.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        Ok(Value::Bool(first > second))
    }
    fn name(&self) -> &str { "GreaterThan" }
}

struct LessThanOp;

#[async_trait]
impl BuiltinOp for LessThanOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let first = inputs.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let second = inputs.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        Ok(Value::Bool(first < second))
    }
    fn name(&self) -> &str { "LessThan" }
}

// ============================================================================
// Logic Operations
// ============================================================================

struct AndOp;

#[async_trait]
impl BuiltinOp for AndOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let result = inputs.iter().all(|v| v.is_truthy());
        Ok(Value::Bool(result))
    }
    fn name(&self) -> &str { "And" }
}

struct OrOp;

#[async_trait]
impl BuiltinOp for OrOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let result = inputs.iter().any(|v| v.is_truthy());
        Ok(Value::Bool(result))
    }
    fn name(&self) -> &str { "Or" }
}

struct NotOp;

#[async_trait]
impl BuiltinOp for NotOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().map(|v| v.is_truthy()).unwrap_or(false);
        Ok(Value::Bool(!input))
    }
    fn name(&self) -> &str { "Not" }
}

struct IfOp;

#[async_trait]
impl BuiltinOp for IfOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let condition = inputs.first().map(|v| v.is_truthy()).unwrap_or(false);
        let then_value = inputs.get(1).cloned().unwrap_or(Value::Null);
        let else_value = inputs.get(2).cloned().unwrap_or(Value::Null);
        Ok(if condition { then_value } else { else_value })
    }
    fn name(&self) -> &str { "If" }
}

// ============================================================================
// Crypto Operations
// ============================================================================

struct HashOp;

#[async_trait]
impl BuiltinOp for HashOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        for input in inputs {
            let bytes = match input {
                Value::String(s) => s.into_bytes(),
                Value::Bytes(b) => b,
                other => serde_json::to_vec(&other).unwrap_or_default(),
            };
            hasher.update(&bytes);
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(Value::Hash(hash))
    }
    fn name(&self) -> &str { "Hash" }
}

struct SignOp;

#[async_trait]
impl BuiltinOp for SignOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        // Simplified signing - in production, use proper key management
        let message = inputs.first().cloned().unwrap_or(Value::Null);
        let message_bytes = serde_json::to_vec(&message).unwrap_or_default();
        
        // For now, return a placeholder signature
        // Real implementation would use ed25519-dalek
        let mut signature = [0u8; 64];
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(&message_bytes);
        signature[..32].copy_from_slice(&hash);
        
        Ok(Value::Bytes(signature.to_vec()))
    }
    fn name(&self) -> &str { "Sign" }
}

struct VerifyOp;

#[async_trait]
impl BuiltinOp for VerifyOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        // Simplified verification
        let _message = inputs.first().cloned().unwrap_or(Value::Null);
        let _signature = inputs.get(1).cloned().unwrap_or(Value::Null);
        
        // Real implementation would verify ed25519 signature
        Ok(Value::Bool(true))
    }
    fn name(&self) -> &str { "Verify" }
}

// ============================================================================
// Time Operations
// ============================================================================

struct TimestampOp;

#[async_trait]
impl BuiltinOp for TimestampOp {
    async fn execute(&self, _inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(Value::Int(timestamp))
    }
    fn name(&self) -> &str { "Timestamp" }
}

// ============================================================================
// AI/Classification Operations
// ============================================================================

struct ClassifyIntentOp;

#[async_trait]
impl BuiltinOp for ClassifyIntentOp {
    async fn execute(&self, inputs: Vec<Value>, params: &serde_json::Value) -> Result<Value, GatewayError> {
        let input = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let _classes: Vec<&str> = params.get("classes")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        
        // Simple heuristic classification
        let intent = if input.starts_with("hi") || input.starts_with("hello") || input.starts_with("hey") {
            "greeting"
        } else if input.contains('?') {
            "question"
        } else if input.starts_with("please") || input.contains("can you") || input.contains("could you") {
            "request"
        } else {
            "statement"
        };
        
        Ok(Value::String(intent.to_string()))
    }
    fn name(&self) -> &str { "ClassifyIntent" }
}

// ============================================================================
// State Operations
// ============================================================================

struct LoadStateOp;

#[async_trait]
impl BuiltinOp for LoadStateOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let session_id = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        // In production, this would load from a state store
        // For now, return an empty map
        let mut state = HashMap::new();
        state.insert("session_id".to_string(), Value::String(session_id.to_string()));
        state.insert("trust_score".to_string(), Value::Confidence(0.5));
        state.insert("message_count".to_string(), Value::Int(0));
        Ok(Value::Map(state))
    }
    fn name(&self) -> &str { "LoadState" }
}

struct SaveStateOp;

#[async_trait]
impl BuiltinOp for SaveStateOp {
    async fn execute(&self, inputs: Vec<Value>, _params: &serde_json::Value) -> Result<Value, GatewayError> {
        let _session_id = inputs.first().and_then(|v| v.as_string()).unwrap_or("");
        let state = inputs.get(1).cloned().unwrap_or(Value::Null);
        // In production, this would save to a state store
        Ok(state)
    }
    fn name(&self) -> &str { "SaveState" }
}

// ============================================================================
// Helpers
// ============================================================================

fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(a) => Value::Array(a.iter().map(json_to_value).collect()),
        serde_json::Value::Object(o) => {
            let map: HashMap<String, Value> = o.iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Map(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_identity() {
        let op = IdentityOp;
        let result = op.execute(vec![Value::String("hello".to_string())], &serde_json::json!({})).await.unwrap();
        assert_eq!(result, Value::String("hello".to_string()));
    }

    #[tokio::test]
    async fn test_starts_with() {
        let op = StartsWithOp;
        let result = op.execute(
            vec![Value::String("/help".to_string())],
            &serde_json::json!({"prefix": "/"}),
        ).await.unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[tokio::test]
    async fn test_extract_first_word() {
        let op = ExtractFirstWordOp;
        let result = op.execute(
            vec![Value::String("/help me please".to_string())],
            &serde_json::json!({}),
        ).await.unwrap();
        assert_eq!(result, Value::String("/help".to_string()));
    }

    #[tokio::test]
    async fn test_multiply() {
        let op = MultiplyOp;
        let result = op.execute(
            vec![Value::Float(0.9), Value::Float(0.8)],
            &serde_json::json!({}),
        ).await.unwrap();
        assert!((result.as_float().unwrap() - 0.72).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_hash() {
        let op = HashOp;
        let result = op.execute(
            vec![Value::String("hello".to_string())],
            &serde_json::json!({}),
        ).await.unwrap();
        matches!(result, Value::Hash(_));
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = BuiltinRegistry::new();
        assert!(registry.len() > 20);
        assert!(registry.get("Identity").is_some());
        assert!(registry.get("StartsWith").is_some());
        assert!(registry.get("Hash").is_some());
    }
}
