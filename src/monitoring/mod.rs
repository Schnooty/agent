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
    redis: RedisMonitor,
}

impl MonitorFutureMaker {
    pub fn new() -> Self {
        Self {
            http: HttpMonitor { },
            process: ProcessMonitor { },
            tcp: TcpMonitor { },
            redis: RedisMonitor { }
        }
    }
}

impl Monitoring for MonitorFutureMaker {
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture {
        match monitor.type_ {
            models::MonitorType::HTTP => self.http.monitor(monitor),
            models::MonitorType::PROCESS=> self.process.monitor(monitor),
            models::MonitorType::TCP => self.tcp.monitor(monitor),
            models::MonitorType::REDIS => self.redis.monitor(monitor),
        }
    }
}

pub struct MonitorStatusBuilder {
    monitor_name: String,
    monitor_type: models::MonitorType,
    timestamp: DateTime<Utc>,
    description: String,
    log: Vec<models::MonitorStatusLogEntry>,
}

impl MonitorStatusBuilder {
    pub fn new<S: ToString>(monitor_name: S, monitor_type: models::MonitorType, timestamp: DateTime<Utc>) -> Self {
        Self {
            monitor_name: monitor_name.to_string(),
            monitor_type,
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
            monitor_name: self.monitor_name.to_owned(),
            status_id: self.monitor_name, // TODO
            status: models::MonitorStatusIndicator::OK,
            monitor_type: self.monitor_type,
            timestamp: self.timestamp ,
            expires_at: self.timestamp + chrono::Duration::days(1),
            expected_result: expected.to_string(),
            actual_result: actual.to_string(),
            description: self.description,
            session: None,
            log: self.log,
        }
    }

    pub fn down<S: ToString, T: ToString>(self, expected: T, actual: S) -> models::MonitorStatus {
        debug!("Monitor is down");
        debug!("Expected result: {}", expected.to_string());
        debug!("Actual result: {}", actual.to_string());

        models::MonitorStatus {
            monitor_name: self.monitor_name.to_owned(),
            status_id: self.monitor_name, // TODO
            monitor_type: self.monitor_type,
            status: models::MonitorStatusIndicator::DOWN,
            timestamp: self.timestamp,
            expires_at: self.timestamp + chrono::Duration::days(1),
            expected_result: expected.to_string(),
            actual_result: actual.to_string(),
            description: self.description,
            log: self.log,
            session: None // TODO
        }
    }
}

impl fmt::Write for MonitorStatusBuilder {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        trace!("Writing log item: {}", s);
        for value in s.split(|c| c == '\n') {
            self.log.push(models::MonitorStatusLogEntry {
                timestamp: Utc::now(),
                value: value.trim().to_owned() 
            });
        }

        Ok(())
    }
}
