use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: Option<String>,
    #[serde(default = "default_group_id")]
    pub group_id: String,
    #[serde(default = "default_api_key")]
    pub api_key: Option<String>,
    #[serde(default = "default_monitor_file")]
    pub monitor_file: Option<String>,
    #[serde(default = "default_alert_file")]
    pub alert_file: Option<String>
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: default_base_url(),
            group_id: default_group_id(),
            api_key: default_api_key(),
            monitor_file: default_monitor_file(),
            alert_file: default_monitor_file(),
        }
    }
}

fn default_base_url() -> Option<String> {
    //"https://api.schnooty.com/".to_owned()
    None
}

fn default_group_id() -> String {
    "main".to_owned()
}

fn default_api_key() -> Option<String> {
    //String::new()
    None
}

fn default_monitor_file() -> Option<String> {
    None
}

fn default_alert_file() -> Option<String> {
    None
}
