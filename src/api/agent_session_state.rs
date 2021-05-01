use chrono::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AgentSessionState {
    pub agent_session_id: String,
    pub monitors: Vec<String>,
    pub heartbeat_due_by: DateTime<Utc>
}
