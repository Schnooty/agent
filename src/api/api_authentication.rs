use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiAuthentication {
    //User { username: String, password: String },
    //ApiKey { api_key: String },
    AgentApiKey { api_key: String },
}
