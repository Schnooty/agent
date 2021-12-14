#[allow(unused_must_use)]
use crate::error::Error;
use crate::monitoring::MonitorFuture;
use crate::monitoring::MonitorSource;
use crate::monitoring::MonitorStatusBuilder;
use chrono::prelude::*;
use openapi_client::models;
use std::fmt::Write;
//use http::request::Request;
//use http::method::Method;
use crate::http::HttpClient;

pub struct HttpMonitor;

impl MonitorSource for HttpMonitor {
    fn type_name(&self) -> &'static str {
        "http"
    }

    #[allow(unused_must_use)]
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture {
        let monitor = monitor.clone();

        Box::pin(async {
            /*let monitor_id = match monitor.id {
                Some(m) => m.to_string(),
                None => {
                    return {
                        error!("Monitor has no ID (name={})", monitor.name);

                        Err(Error::new("Could not find the ID for this monitor. This is an internal error."))
                    }
                }
            };*/

            let mut status_builder =
                MonitorStatusBuilder::new(&monitor.name, models::MonitorType::HTTP, Utc::now());

            const EXPECTED: &str = "200-level status code";

            let (request, mut status_builder) = if let (Some(Ok(ref method)), Some(url)) = (
                monitor
                    .body
                    .method
                    .clone()
                    .map(|m| m.parse() as Result<reqwest::Method, _>),
                &monitor.body.url,
            ) {
                let mut builder = reqwest::Client::new().request(method.clone(), url);

                for header in monitor.body.headers.unwrap_or_default().iter() {
                    builder = builder.header(&header.name, &header.value);
                }

                writeln!(status_builder, "Beginning GET request to {}", url.trim());

                let body = match monitor.body.body {
                    Some(ref b) => b.clone(),
                    None => String::new(),
                };

                (
                    builder.body(body).build().unwrap(),
                    status_builder.description(format!("GET {} has success status code", url)),
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

                writeln!(
                    result_builder,
                    "Monitor configuration (method={}, url={})",
                    method, url
                );

                return Ok(result_builder.down(
                    EXPECTED,
                    "Either method or url is missing in this monitor's configuration, or both",
                ));
            };

            let client = HttpClient::new(request);

            let response_result = client.send().await;

            let response = match response_result {
                Ok(response) => response,
                Err(err) => {
                    writeln!(status_builder, "Error completing HTTP request: {}", err);
                    return Err(err.into());
                }
            };

            writeln!(
                status_builder,
                "Response status code: {}",
                response.status().to_string().trim()
            );

            if response.status().is_success() {
                writeln!(status_builder, "Response status code is success.\n All OK");

                Ok(status_builder.ok(EXPECTED, response.status()))
            } else {
                Ok(status_builder.down(EXPECTED, response.status()))
            }
        })
    }
}
