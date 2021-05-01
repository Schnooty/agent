use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_base_uri")]
    pub base_uri: String,
    pub group_id: String,
    pub api_key: String
}

fn default_base_uri() -> String {
    "https://api.schnooty.com/".to_owned()
}
