use openapi_client::models;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: Option<String>,
    #[serde(default = "default_api_key")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub monitors: Vec<models::Monitor>,
    #[serde(default)]
    pub alerts: Vec<models::Alert>,
    pub status: StatusSink,
    pub session: SessionInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MonitorSource {
    #[serde(rename = "file")]
    File { path: String },
    #[serde(rename = "api")]
    Api {
        base_url: Option<String>,
        api_key: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AlertSource {
    #[serde(rename = "file")]
    File { path: String },
    #[serde(rename = "api")]
    Api {
        base_url: Option<String>,
        api_key: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusSink {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub name: String,
    pub enabled: bool,
}

/*impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: default_base_url(),
            group_id: default_group_id(),
            api_key: default_api_key(),
            monitor_file: default_monitor_file(),
            alert_file: default_monitor_file(),
        }
    }
}*/

fn default_base_url() -> Option<String> {
    //"https://api.schnooty.com/".to_owned()
    None
}

fn default_api_key() -> Option<String> {
    //String::new()
    None
}
