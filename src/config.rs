use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Monitor {
    #[serde(rename = "id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(rename = "type")]
    pub type_: MonitorType,

    #[serde(rename = "enabled")]
    pub enabled: bool,

    #[serde(rename = "name")]
    pub name: String,

    /// Describes what this monitor checks.
    #[serde(rename = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "period")]
    pub period: String,

    #[serde(rename = "timeout")]
    pub timeout: String,

    #[serde(rename = "body")]
    pub body: MonitorBody,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Session {
    #[serde(rename = "name")]
    pub name: String,

    #[serde(rename = "hostname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    #[serde(rename = "platform")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<models::PlatformInfo>,

    /// UTC UNIX timestamp in with fractional offset.
    #[serde(rename = "lastUpdated")]
    pub last_updated: chrono::DateTime<chrono::Utc>,

    /// UTC UNIX timestamp in with fractional offset.
    #[serde(rename = "startedAt")]
    pub started_at: chrono::DateTime<chrono::Utc>,
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MonitorStatus {
    #[serde(rename = "statusId")]
    pub status_id: String,

    #[serde(rename = "monitorType")]
    pub monitor_type: models::MonitorType,

    #[serde(rename = "monitorName")]
    pub monitor_name: String,

    #[serde(rename = "status")]
    pub status: models::MonitorStatusIndicator,

    /// UTC UNIX timestamp in with fractional offset.
    #[serde(rename = "timestamp")]
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// UTC UNIX timestamp in with fractional offset.
    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "expectedResult")]
    pub expected_result: String,

    #[serde(rename = "actualResult")]
    pub actual_result: String,

    #[serde(rename = "description")]
    pub description: String,

    #[serde(rename = "session")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<models::Session>,

    #[serde(rename = "log")]
    pub log: Vec<models::MonitorStatusLogEntry>,
}


#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum MonitorType {
    #[serde(rename = "http")]
    HTTP,
    #[serde(rename = "process")]
    PROCESS,
    #[serde(rename = "tcp")]
    TCP,
    #[serde(rename = "redis")]
    REDIS,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MonitorBody {
    #[serde(rename = "url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(rename = "method")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    #[serde(rename = "headers")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<models::HttpHeader>>,

    #[serde(rename = "body")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// The name of the executable process to be monitored.
    #[serde(rename = "executable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,

    /// If true, the process(s) will be located by the full path of the executable e.g. /usr/bin/node
    #[serde(rename = "isPathAbsolute")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_path_absolute: Option<bool>,

    /// The minimum number of processes that match the executable.
    #[serde(rename = "minimumCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_count: Option<isize>,

    /// The maximum number of processes that match the executable.
    #[serde(rename = "maximumCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_count: Option<isize>,

    #[serde(rename = "maximumRamIndividual")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_ram_individual: Option<String>,

    #[serde(rename = "maximumRamTotal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_ram_total: Option<String>,

    #[serde(rename = "hostname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    #[serde(rename = "port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    #[serde(rename = "db")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db: Option<isize>,

    #[serde(rename = "username")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(rename = "password")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    #[serde(rename = "constraints")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<models::FieldConstraint>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Alert {
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: Option<String>,
    #[serde(default = "default_api_key")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub monitors: Vec<Monitor>,
    #[serde(default)]
    pub alerts: Vec<Alert>,
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub create_session: bool,
    #[serde(default)]
    pub upload_statuses: bool,
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
