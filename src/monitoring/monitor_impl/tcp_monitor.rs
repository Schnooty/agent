#[allow(unused_must_use)]
use crate::error::Error;
use crate::monitoring::MonitorFuture;
use crate::monitoring::MonitorSource;
use crate::monitoring::MonitorStatusBuilder;
use crate::openapi_client::models;
use async_std::net::TcpStream;
use chrono::prelude::*;
use std::fmt::Write;

pub struct TcpMonitor;

impl MonitorSource for TcpMonitor {
    fn type_name(&self) -> &'static str {
        "tcp"
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

                        Err(Error::new(
                            "Could not find the ID for this monitor. This is an internal error.",
                        ))
                    }
                }
            };
            let mut result_builder =
                MonitorStatusBuilder::new(&monitor_id, models::MonitorType::TCP, Utc::now());

            writeln!(result_builder, "Checking monitor configuration");

            result_builder =
                result_builder.description("Connection to host is successful over TCP".to_string());

            let hostname_port = match (monitor.body.hostname, monitor.body.port) {
                (Some(h), Some(p)) => format!("{}:{}", h, p),
                _ => {
                    writeln!(result_builder, "Monitor is missing hostname, port, or both");
                    return Ok(result_builder.down(
                        "Successful connection over TCP",
                        "Monitor is misconfigured. Please check it has both a hostname and port set",
                    ));
                }
            };

            let expected = format!("Successful connection to {} over TCP", hostname_port);

            writeln!(result_builder, "Opening connection to {}", hostname_port);

            match TcpStream::connect(&hostname_port).await {
                Ok(conn) => {
                    writeln!(result_builder, "Connection successfully established.");
                    drop(conn);
                    Ok(result_builder.ok(expected, "Connection was successful"))
                }
                Err(err) => {
                    writeln!(result_builder, "Error connecting to {}", hostname_port);
                    Ok(result_builder.down(expected, format!("Failed to connect: {}", err)))
                }
            }
        })
    }
}
