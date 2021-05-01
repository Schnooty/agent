#[allow(unused_must_use)]
use crate::error::Error;
use crate::monitoring::MonitorSource;
use crate::monitoring::MonitorStatusBuilder;
use crate::monitoring::MonitorFuture;
use chrono::prelude::*;
use openapi_client::models;
use reqwest::redirect::Policy;
use reqwest::Client;
use reqwest::Method;
use std::fmt::Write;

pub struct HttpMonitor;

impl MonitorSource for HttpMonitor {
    fn type_name(&self) -> &'static str {
        "http"
    }

    #[allow(unused_must_use)]
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture {
        let monitor = monitor.clone();

        Box::pin(async {
            let monitor_id = match monitor.id {
                Some(m) => m.to_string(),
                None => {
                    return {
                        error!("Monitor has no ID (name={})", monitor.name);

                        Err(Error::new("Could not find the ID for this monitor. This is an internal error."))
                    }
                }
            };

            let status_builder = MonitorStatusBuilder::new(&monitor_id, Utc::now());

            const EXPECTED: &str = "200-level status code";

            let (builder, mut status_builder, url) = if let (Some(Ok(ref method)), Some(url)) = (
                monitor
                    .body
                    .method
                    .clone()
                    .map(|m| m.parse() as Result<Method, _>),
                &monitor.body.url,
            ) {
                let mut builder = match Client::builder().redirect(Policy::none()).build() {
                    Ok(b) => b.request(method.clone(), url),
                    Err(err) => return Err(Error::new(err)),
                };
                for header in monitor.body.headers.unwrap_or_default().iter() {
                    builder = builder.header(&header.name, &header.value);
                }
                (
                    builder,
                    status_builder.description(format!("GET {} has success status code", url)),
                    url
                )
            } else {
                let mut result_builder =
                    status_builder.description("HTTP monitor is missing configuration".to_string());

                let method = match monitor.body.method {
                    Some(m) => m,
                    None => "<missing>".to_owned(),
                };
                let url = match monitor.body.url {
                    Some(u) => u,
                    None => "<missing>".to_owned(),
                };

                writeln!(result_builder, "Monitor configuration (method={}, url={})", method, url);

                return Ok(result_builder.down(
                    EXPECTED,
                    "Either method or url is missing in this monitor's configuration, or both",
                ));
            };

            writeln!(status_builder, "Beginning GET request to {}", url.trim());

            let response_result = builder.send().await;

            let response = match response_result {
                Ok(response) => response,
                Err(err) => {
                    writeln!(status_builder, "Error completing HTTP request: {}", err);
                    return Err(Error::from(err));
                }
            };

            writeln!(status_builder, "Response status code: {}", response.status().to_string().trim());

            if response.status().is_success() {
                writeln!(status_builder, "Response status code is success.\n All OK");

                Ok(status_builder.ok(EXPECTED, response.status()))
            } else {
                Ok(status_builder.down(EXPECTED, response.status()))
            }
        })
    }
}
