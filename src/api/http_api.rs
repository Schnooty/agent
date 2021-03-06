use crate::api::{Api, ApiFuture, ReadApi};
use crate::error::Error;
use crate::http::HttpClient;
use crate::openapi_client::models;
use chrono::DateTime;
use chrono::Utc;
use hostname::get as get_hostname;

#[derive(Clone, Debug)]
pub struct HttpConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Clone)]
pub struct HttpApi {
    config: HttpConfig,
    options: HttpApiOptions,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
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
            started_at: Utc::now(),
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
                    None => String::new(),
                };
                let password: String = match iter.next() {
                    Some(ref p) => p.to_string(),
                    None => String::new(),
                };
                Some((username, Some(password)))
            }
            None => None,
        }
    }
}

impl ReadApi for HttpApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>> {
        let basic_auth = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}monitors", self.config.base_url.to_string());

        let mut request = reqwest::Client::new().request(reqwest::Method::GET, uri);

        if let Some((agent_id, Some(password))) = basic_auth {
            let base64_creds = base64::encode(format!("{}:{}", agent_id, password));

            request = request.header("Authorization", format!("Basic {}", base64_creds));
        }

        Box::pin(async {
            let client = HttpClient::new(request.body(String::new()).build().unwrap());

            let response_result = client.send().await;

            //let response_body_result: Result<models::MonitorArray, HttpError> =
            match response_result {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("Response status was NOT success status code");
                        return Err(Error::new(format!(
                            "Agent failed to get monitors. Got status code {}",
                            response.status()
                        )));
                    }

                    Ok(serde_json::from_slice(&response.bytes().await?)?)
                }
                Err(err) => {
                    return Err(Error::new(format!("Agent failed to get monitors: {}", err)))
                }
            }

            /*match response_body_result {
                Ok(body) => {
                    debug!("Retrieved {} monitors", body.monitors.len());
                    Ok(body.monitors)
                }
                Err(err) => Err(Error::from(err)),
            }*/
        })
    }

    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>> {
        let basic_auth = self.get_basic_auth();
        debug!("Getting monitors");

        let uri = format!("{}alerts", self.config.base_url.to_string());

        let mut builder = reqwest::Client::new().request(reqwest::Method::GET, uri);

        if let Some((agent_id, Some(password))) = basic_auth {
            let base64_creds = base64::encode(format!("{}:{}", agent_id, password));

            builder = builder.header("Authorization", format!("Basic {}", base64_creds));
        }

        Box::pin(async move {
            let client = HttpClient::new(builder.body(String::new()).build().unwrap());
            let response_result = client.send().await;

            let response_body_result: Result<models::AlertArray, _> = match response_result {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("Response status was NOT success status code");
                        return Err(Error::new(format!(
                            "Agent failed to get alerts. Got status code {}",
                            response.status()
                        )));
                    }

                    serde_json::from_slice(&response.bytes().await?)
                }
                Err(err) => return Err(Error::new(format!("Agent failed to get alerts: {}", err))),
            };

            match response_body_result {
                Ok(body) => Ok(body.alerts),
                Err(err) => Err(Error::new(format!("Agent failed to get alerts: {}", err))),
            }
        })
    }
}

impl Api for HttpApi {
    fn post_heartbeat(&mut self, session_name: &str) -> ApiFuture<models::Session> {
        let basic_auth = self.get_basic_auth();

        debug!("Posting heartbeat (session_name={})", session_name);

        let uri = format!(
            "{}sessions/{}",
            self.config.base_url.to_string(),
            session_name
        );

        debug!("Building heartbeat request (uri={})", uri);

        let mut builder = reqwest::Client::new()
            .request(reqwest::Method::PUT, uri)
            .header("Content-Type", "application/json");

        if let Some((agent_id, Some(password))) = basic_auth {
            let base64_creds = base64::encode(format!("{}:{}", agent_id, password));

            builder = builder.header("Authorization", format!("Basic {}", base64_creds));
        }

        let hostname = match get_hostname() {
            Ok(h) => h.to_string_lossy().into_owned(),
            Err(e) => format!("Error getting hostname: {}", e),
        };

        let platform = models::PlatformInfo {
            os: Some(std::env::consts::OS.to_string()),
            cpu: None,
        };

        let body = serde_json::to_string(&models::Session {
            name: session_name.to_owned(),
            hostname: Some(hostname),
            platform: Some(platform),
            last_updated: Utc::now(),
            started_at: self.started_at,
        })
        .unwrap(); // TODO

        //client = client.json()
        //    .timeout(StdDuration::from_secs(self.options.timeout_seconds));

        let client = HttpClient::new(builder.body(body).build().unwrap()); // TODO

        let _agent_session_id = session_name.to_owned();

        Box::pin(async move {
            debug!("Sending heartbeat");

            let result = client.send().await;

            debug!("Heartbeat send complete");

            let response = match result {
                Ok(r) => r,
                Err(err) => {
                    error!("Error communicating with API: {:?}", err);

                    return Err(Error::new(format!(
                        "Agent failed to send heartbeat: {}",
                        err
                    )));
                }
            };

            debug!("Received response: {:?}", response);

            if !response.status().is_success() {
                warn!("Response status was NOT success status code");
                return Err(Error::new(format!(
                    "Agent failed to send heartbeat. Got status code {}",
                    response.status()
                )));
            }

            debug!("Loading JSON data");

            let response_json: serde_json::Result<models::SessionContainer> =
                serde_json::from_slice(&response.bytes().await?);

            match response_json {
                Ok(s) => Ok(s.session),
                Err(e) => Err(Error::new(format!(
                    "Agent failed to load the agent list from API: {}",
                    e
                ))),
            }
        })
    }

    fn post_statuses(&mut self, statuses: &[models::MonitorStatus]) -> ApiFuture<()> {
        let basic_auth = self.get_basic_auth();
        let statuses: Vec<_> = statuses.to_vec();
        let base_url = self.config.base_url.to_string();

        debug!("Uploading {} monitor status(es)", statuses.len());

        Box::pin(async move {
            for status in statuses.iter() {
                let uri = format!("{}statuses/{}", base_url, status.status_id);

                let mut builder = reqwest::Client::new().request(reqwest::Method::POST, uri);

                if let Some((ref agent_id, Some(ref password))) = basic_auth {
                    let base64_creds = base64::encode(format!("{}:{}", agent_id, password));

                    builder = builder.header("Authorization", format!("Basic {}", base64_creds));
                    //client = client.basic_auth(&agent_id, password);
                }

                let request = builder.body(serde_json::to_string(&status).unwrap()); // TODO
                let client = HttpClient::new(request.build().unwrap());

                match client.send().await {
                    Ok(r) => r,
                    Err(err) => {
                        error!("Failed to upload results: {}", err);
                        return Err(Error::new(format!("Failed to upload results: {}", err)));
                    }
                };
            }

            Ok(())
        })
    }
}
