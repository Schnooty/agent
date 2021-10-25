use chrono::Utc;
use chrono::DateTime;
use crate::api::{Api, ApiFuture};
use crate::error::Error;
use openapi_client::models;
use hostname::get as get_hostname;

use reqwest::Client;
use std::time::Duration as StdDuration;

#[derive(Clone)]
pub struct HttpConfig {
    pub base_url: String,
    pub api_key: Option<String>
}

pub struct HttpApi {
    config: HttpConfig,
    options: HttpApiOptions,
    started_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct HttpApiOptions {
    timeout_seconds: u64,
}

impl Default for HttpApiOptions {
    fn default() -> Self {
        HttpApiOptions {
            timeout_seconds: 30,
        }
    }
}

#[allow(dead_code)]
impl HttpApi {
    pub fn new(config: &HttpConfig) -> Self {
        Self {
            config: config.clone(),
            options: Default::default(),
            started_at: Utc::now()
        }
    }

    pub fn options(&mut self, options: HttpApiOptions) {
        self.options = options;
    }

    fn get_basic_auth(&self) -> Option<(String, Option<String>)> {
        match &self.config.api_key {
            Some(ref api_key) => {
                let mut iter = api_key.split(':');
                let username: String = match iter.next() {
                    Some(ref u) => u.to_string(),
                    None => String::new()
                };
                let password: String = match iter.next() {
                    Some(ref p) => p.to_string(),
                    None => String::new()
                };
                Some((username, Some(password)))
            },
            None => None
        }
    }
}

impl Api for HttpApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>> {
        let basic_auth = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}monitors", self.config.base_url.to_string());

        let mut client = Client::new()
            .get(&uri);

        if let Some((agent_id, password)) = basic_auth {
            client = client.basic_auth(&agent_id, password);
        }
            
        client = client.timeout(StdDuration::from_secs(self.options.timeout_seconds));

        Box::pin(async {
            let response_result = client.send().await;

            let response_body_result: Result<models::MonitorArray, _> = match response_result {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("Response status was NOT success status code");
                        return Err(Error::new(format!("Agent failed to get monitors. Got status code {}", response.status())));
                    }

                    response.json().await
                }
                Err(err) => return Err(Error::new(format!("Agent failed to get monitors: {}", err))),
            };

            match response_body_result {
                Ok(body) => {
                    debug!("Retrieved {} monitors", body.monitors.len());
                    Ok(body.monitors)
                }
                Err(err) => Err(Error::from(err)),
            }
        })
    }

    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>> {
        let basic_auth  = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}alerts", self.config.base_url.to_string());

        let mut client = Client::new()
            .get(&uri);
        if let Some((agent_id, password)) = basic_auth {
            client = client.basic_auth(&agent_id, password);
        }
        client = client.timeout(StdDuration::from_secs(self.options.timeout_seconds));

        Box::pin(async {
            let response_result = client.send().await;

            let response_body_result: Result<models::AlertArray, _> = match response_result {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("Response status was NOT success status code");
                        return Err(Error::new(format!("Agent failed to get alerts. Got status code {}", response.status())));
                    }

                    response.json().await
                }
                Err(err) => return Err(Error::new(format!("Agent failed to get alerts: {}", err)))
            };

            match response_body_result {
                Ok(body) => Ok(body.alerts),
                Err(err) => Err(Error::new(format!("Agent failed to get alerts: {}", err)))
            }
        })
    }

    fn post_heartbeat(&mut self, group_id: &str, session_name: &str) -> ApiFuture<models::Session> {
        let basic_auth = self.get_basic_auth();

        debug!(
            "Posting heartbeat (session_name={})",
            session_name
        );

        let uri = format!(
            "{}sessions/{}",
            self.config.base_url.to_string(),
            group_id
        );

        debug!("Building heartbeat request (uri={})", uri);

        let mut client = Client::new()
            .post(&uri);

        if let Some((agent_id, password)) = basic_auth {
            client = client.basic_auth(&agent_id, password);
        }

        let hostname = match get_hostname() {
            Ok(h) => h.to_string_lossy().into_owned(),
            Err(e) => format!("Error getting hostname: {}", e)
        };

        let platform = models::PlatformInfo {
            os: Some(std::env::consts::OS.to_string()),
            cpu: None 
        };

        client = client.json(&models::Session {
                name: session_name.to_owned(),
                hostname: Some(hostname),
                platform: Some(platform),
                last_updated: Utc::now(),
                started_at: self.started_at
            })
            .timeout(StdDuration::from_secs(self.options.timeout_seconds));

        let _agent_session_id = session_name.to_owned();

        Box::pin(async move {
            debug!("Sending heartbeat");
            let result = client.send().await;

            let response = match result {
                Ok(r) => r,
                Err(err) => {
                    error!("Error communicating with API: {:?}", err);

                    return Err(Error::new(format!("Agent failed to send heartbeat: {}", err)));
                }
            };

            debug!("Received response: {:?}", response);

            if !response.status().is_success() {
                warn!("Response status was NOT success status code");
                return Err(Error::new(format!("Agent failed to send heartbeat. Got status code {}", response.status())));
            }

            debug!("Loading JSON data");

            let response_json: Result<models::Session, _> = response.json().await;

            match response_json {
                Ok(r) => Ok(r),
                Err(e) => Err(Error::new(format!("Agent failed to load the agent list from API: {}", e)))
            }
        })
    }

    fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> ApiFuture<()> {
        let basic_auth = self.get_basic_auth();
        let statuses: Vec<_> = statuses.to_vec();

        debug!("Uploading {} monitor status(es)", statuses.len());

        let uri = format!("{}statuses", self.config.base_url.to_string());

        let body = models::MonitorStatusArray {
            statuses
        };

        let mut client = Client::new()
            .post(&uri);
        if let Some((agent_id, password)) = basic_auth {
            client = client.basic_auth(&agent_id, password);
        }
        client = client
            .json(&body)
            .timeout(StdDuration::from_secs(self.options.timeout_seconds));

        Box::pin(async move {
            let result = match client.send().await {
                Ok(r) => r,
                Err(err) => {
                    error!("Failed to upload results: {}", err);
                    return Err(Error::new(format!("Failed to upload results: {}", err)));
                }
            };

            let status = result.status();

            if !status.is_success() {
                error!("Error uploading statuses. Got status code: {}", status);

                let status_error = Error::new(format!("Failed to upload results. Got status code: {}", status));

                let errors: Vec<models::ResponseError> = match result.json().await {
                    Ok(e) => e,
                    Err(err) => {
                        error!("Failed to parse error response: {}", err);
                        return Err(status_error)
                    }
                };

                for error in errors {
                    error!("Got {} error: {}", error.error_code, error.error_message);
                }

                return Err(status_error);
            }

            // TODO use the result

            Ok(())

        })
    }
}
