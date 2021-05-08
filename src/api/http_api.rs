use chrono::Duration;
use chrono::Utc;
use crate::api::{Api, ApiFuture};
use crate::error::Error;
use crate::config::Config;
use openapi_client::models;
use crate::api::AgentSessionState;

use reqwest::Client;
use std::time::Duration as StdDuration;

pub struct HttpApi {
    config: Config,
    options: HttpApiOptions,
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
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.to_owned(),
            options: Default::default()
        }
    }

    pub fn options(&mut self, options: HttpApiOptions) {
        self.options = options;
    }

    fn get_basic_auth(&self) -> (String, Option<String>) {
        let mut iter = self.config.api_key.split(':');
        let username: String = match iter.next() {
            Some(ref u) => u.to_string(),
            None => String::new()
        };
        let password: String = match iter.next() {
            Some(ref p) => p.to_string(),
            None => String::new()
        };
        (username, Some(password))
    }
}

impl Api for HttpApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>> {
        let (agent_id, password) = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}monitors", self.config.base_url.to_string());

        let client = Client::new()
            .get(&uri)
            .basic_auth(&agent_id, password)
            .timeout(StdDuration::from_secs(self.options.timeout_seconds));

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
        let (agent_id, password) = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}alerts", self.config.base_url.to_string());

        let client = Client::new()
            .get(&uri)
            .basic_auth(&agent_id, password)
            .timeout(StdDuration::from_secs(self.options.timeout_seconds));

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

    fn post_heartbeat(&mut self, group_id: &str, session_id: &str) -> ApiFuture<AgentSessionState> {
        let (agent_id, password) = self.get_basic_auth();

        debug!(
            "Posting heartbeat for session {} in group {}",
            session_id, group_id
        );

        let uri = format!(
            "{}session/{}",
            self.config.base_url.to_string(),
            group_id
        );

        debug!("Building heartbeat request (uri={}, agent_id={}, password={:?})", uri, agent_id, password);

        let client = Client::new()
            .post(&uri)
            .basic_auth(&agent_id, password)
            .json(&models::AgentSessionRequest {
                session_id: session_id.to_owned(),
                is_new: Some(true),
            })
            .timeout(StdDuration::from_secs(self.options.timeout_seconds));

        let agent_session_id = session_id.to_owned();

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

            let response_json: Result<models::AgentSessionState, _> = response.json().await;

            match response_json {
                Ok(r) => {
                    if let Some(agents) = r.agents {
                        if let Some(this_agent) = agents
                            .into_iter().find(|a| a.session_id == agent_session_id)
                        {
                            Ok(AgentSessionState {
                                agent_session_id,
                                monitors: this_agent
                                    .monitor_ids
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect(),
                                heartbeat_due_by: Utc::now() + Duration::minutes(2),
                            })
                        } else {
                            Err(Error::new("Could not find this agent in the list from API"))
                        }
                    } else {
                        Err(Error::new("Could not find this agent in the list from API"))
                    }
                }
                Err(e) => Err(Error::new(format!("Agent failed to load the agent list from API: {}", e)))
            }
        })
    }

    fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> ApiFuture<()> {
        let (agent_id, password) = self.get_basic_auth();
        let statuses: Vec<_> = statuses.to_vec();

        debug!("Uploading {} monitor status(es)", statuses.len());

        let uri = format!("{}statuses", self.config.base_url.to_string());

        let body = models::MonitorStatusArray {
            statuses
        };

        let client = Client::new()
            .post(&uri)
            .basic_auth(&agent_id, password)
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
