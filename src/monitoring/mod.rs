mod monitor_impl;

pub use monitor_impl::*;

use chrono::DateTime;
use chrono::offset::Utc;
use crate::error::Error;
use crate::monitoring::{HttpMonitor, MonitorSource, ProcessMonitor};
use log::*;
use openapi_client::models;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

pub type MonitorFuture = Pin<Box<dyn Future<Output = Result<models::MonitorStatus, Error>>>>;

pub trait Monitoring {
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture;
}

pub struct MonitorFutureMaker {
    http: HttpMonitor,
    process: ProcessMonitor,
    tcp: TcpMonitor,
}

impl MonitorFutureMaker {
    pub fn new() -> Self {
        Self {
            http: HttpMonitor { },
            process: ProcessMonitor { },
            tcp: TcpMonitor { }
        }
    }
}

impl Monitoring for MonitorFutureMaker {
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture {
        match monitor.type_ {
            models::MonitorType::HTTP => self.http.monitor(monitor),
            models::MonitorType::PROCESS=> self.process.monitor(monitor),
            models::MonitorType::TCP => self.tcp.monitor(monitor),
        }
    }
}

pub struct MonitorStatusBuilder {
    monitor_id: String,
    timestamp: DateTime<Utc>,
    description: String,
    log: Vec<models::MonitorStatusLogEntry>,
}

impl MonitorStatusBuilder {
    pub fn new<S: ToString>(monitor_id: S, timestamp: DateTime<Utc>) -> Self {
        Self {
            monitor_id: monitor_id.to_string(),
            timestamp,
            description: "Description unavailable".to_owned(),
            log: Vec::new()
        }
    }

    pub fn description<S: ToString>(mut self, description: S) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn ok<S: ToString, T: ToString>(self, expected: T, actual: S) -> models::MonitorStatus {
        debug!("Monitor is OK");
        debug!("Expected result: {}", expected.to_string());
        debug!("Actual result: {}", actual.to_string());

        models::MonitorStatus {
            monitor_id: self.monitor_id,
            status: models::MonitorStatusIndicator::OK,
            timestamp: self.timestamp ,
            last_result: models::MonitorStatusResult {
                expected: expected.to_string(),
                actual: actual.to_string(),
            },
            description: self.description,
            log: Some(self.log),
        }
    }

    pub fn down<S: ToString, T: ToString>(self, expected: T, actual: S) -> models::MonitorStatus {
        debug!("Monitor is down");
        debug!("Expected result: {}", expected.to_string());
        debug!("Actual result: {}", actual.to_string());

        models::MonitorStatus {
            monitor_id: self.monitor_id,
            status: models::MonitorStatusIndicator::DOWN,
            timestamp: self.timestamp,
            last_result: models::MonitorStatusResult {
                expected: expected.to_string(),
                actual: actual.to_string(),
            },
            description: self.description,
            log: Some(self.log),
        }
    }
}

impl fmt::Write for MonitorStatusBuilder {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for value in s.split(|c| c == '\n') {
            self.log.push(models::MonitorStatusLogEntry {
                timestamp: Utc::now(),
                value: value.trim().to_owned() 
            });
        }

        Ok(())
    }
}
