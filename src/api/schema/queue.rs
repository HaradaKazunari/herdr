use serde::{Deserialize, Serialize};

/// Append a prompt to a target agent's next-prompt queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueAddParams {
    pub target: String,
    pub text: String,
}

/// Identify a target agent's next-prompt queue (for list / pop).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueTargetParams {
    pub target: String,
}
