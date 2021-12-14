use crate::error::Error;
use crate::monitoring::MonitorSource;
use crate::monitoring::MonitorStatusBuilder;
use crate::monitoring::MonitorFuture;
use chrono::prelude::*;
use openapi_client::models;
use redis::{ConnectionInfo, RedisConnectionInfo, ConnectionAddr, Client};
use redis;
use std::fmt::Write;

pub struct RedisMonitor;

impl MonitorSource for RedisMonitor {
    fn type_name(&self) -> &'static str {
        "redis"
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

            let mut result_builder = MonitorStatusBuilder::new(&monitor_id, models::MonitorType::REDIS, Utc::now());

            // set up parameters
            let host = monitor.body.hostname.unwrap(); // TODO
            let port = monitor.body.port.unwrap(); // TODO;
            let db = monitor.body.db.map(|i| i as i64).ok_or(Error::new("Redis monitor is missing `db` property"))?;
            let username = monitor.body.username;
            let password = monitor.body.password;
            let constraints = match monitor.body.constraints {
                Some(c) => c,
                None => vec![]
            };

            let connection_info = ConnectionInfo {
                addr: ConnectionAddr::Tcp(host.to_owned(), port),
                redis: RedisConnectionInfo {
                    db,
                    username,
                    password
                }
            };

            // connect to redis    
            writeln!(&mut result_builder, "Opening connection to redis on {}:{}", host, port);
            let mut conn = Client::open(connection_info)?;
            // authenticate if necessary
            /*const AUTH: &'static str = "AUTH";
            match (username, password) {
                (Some(username), None) => {
                    writeln!(&mut result_builder, "Authentication with username only");
                    redis::cmd(AUTH).arg(username).query(&mut conn)?
                },
                (Some(username), Some(password)) => {
                    writeln!(&mut result_builder, "Authentication with username and password");
                    redis::cmd(AUTH).arg(username).arg(password).query(&mut conn)?
                },
                (None, Some(_password)) => {
                    writeln!(&mut result_builder, "Password was provided, but not username. Not AUTHing.");
                },
                (None, None) => {
                    writeln!(&mut result_builder, "No need to AUTH.");
                }
            };*/

            const INFO: &'static str = "INFO";

            writeln!(&mut result_builder, "Loading data using INFO command");
            // load data and parse
            let info_dict: redis::InfoDict = redis::cmd(INFO).query(&mut conn)?;
            
            writeln!(&mut result_builder, "Successfully loaded INFO data. Now checking {} constraints.", constraints.len());
            // build result from constraints
            let failed_constraints: Vec<_> = constraints.iter().filter_map(|constraint| {
                writeln!(
                    &mut result_builder,
                    "Checking if {} {} '{}'",
                    constraint.name,
                    constraint.operator,
                    constraint.value
                );
                let field_value_option: Option<String> = info_dict.get(&constraint.name);
                if let Some(field_value) = field_value_option {
                    // now apply operator
                    let apply = Apply { 
                        operator: constraint.operator
                    };
                    if !apply.apply(&field_value, &constraint.value) {
                        writeln!(&mut result_builder, "Constraint check FAILED. Value of '{}' {} {}", constraint.name, constraint.operator, field_value);
                        Some(constraint.name.to_owned())
                    } else {
                        writeln!(&mut result_builder, "Constraint check OK. Value of '{}' {} {}", constraint.name, constraint.operator, field_value);
                        None
                    }
                } else {
                    writeln!(&mut result_builder, "Failed to find field '{}'", constraint.name);
                    Some(constraint.name.to_owned())
                }
            }).collect();

            Ok(if failed_constraints.len() == 0 {
                result_builder.ok("0 failed constraints", "Zero failed constraints")
            } else {
                result_builder.down("0 failed constraints", "{} failed constraint(s)")
            })
        })
    }
}

/// Implementation of comparison operator for stringly types.
struct Apply {
    operator: models::CmpOperator
}

impl Apply {
    fn apply(&self, lhs: &str, rhs: &str) -> bool {
        match self.operator {
            models::CmpOperator::EQ => lhs == rhs,
            models::CmpOperator::NE => lhs != rhs,
            _ => {
                let l = i64::from_str_radix(lhs, 10).unwrap();// TODO
                let r = i64::from_str_radix(rhs, 10).unwrap();
                match self.operator {
                    models::CmpOperator::LT => l < r,
                    models::CmpOperator::LE => l <= r,
                    models::CmpOperator::GT => l > r,
                    models::CmpOperator::GE => l >= r,
                    _ => unreachable!()
                }
            },
        }
    }
}

