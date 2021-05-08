use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_group_id")]
    pub group_id: String,
    #[serde(default = "default_api_key")]
    pub api_key: String
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: default_base_url(),
            group_id: default_group_id(),
            api_key: default_api_key()
        }
    }
}

fn default_base_url() -> String {
    "https://api.schnooty.com/".to_owned()
}

fn default_group_id() -> String {
    "main".to_owned()
}

fn default_api_key() -> String {
    String::new()
}
