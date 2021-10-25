use crate::error::Error;
use crate::monitoring::MonitorFuture;
use crate::monitoring::MonitorSource;
use crate::monitoring::MonitorStatusBuilder;
use chrono::prelude::*;
use openapi_client::models;
use std::path::Path;
use sysinfo::{ProcessExt, System, SystemExt};
use std::fmt::Write;

pub struct ProcessMonitor;

impl MonitorSource for ProcessMonitor {
    fn type_name(&self) -> &'static str {
        "process"
    }

    #[allow(unused_must_use)]
    fn monitor(&mut self, monitor: &models::Monitor) -> MonitorFuture {
        let monitor = monitor.clone();

        Box::pin(async {
            let monitor_id = match &monitor.id {
                Some(ref id) => id.to_string(),
                None => return Err(Error::new("Could not find the ID for this monitor. This is an internal error.")),
            };
            let absolute_path = match &monitor.body.is_path_absolute {
                Some(true) => true,
                _ => false,
            };
            let builder = MonitorStatusBuilder::new(monitor_id, models::MonitorType::PROCESS, Utc::now());

            let executable_name = match monitor.body.executable {
                Some(e) => e,
                None => {
                    return Ok(builder
                        .description("Process monitor for unknown executable")
                        .down(
                            "Process monitor should have executable path to monitor",
                            "Process monitor's executable field is null",
                        ));
                }
            };

            let mut builder =
                builder.description(format!("Process monitor for {}", executable_name));

            let max_ram_instance = match monitor.body.maximum_ram_individual.map(|m| m.parse()) {
                Some(Ok(m)) => m,
                _ => usize::MAX,
            };

            let max_ram_total = match monitor.body.maximum_ram_total.map(|m| m.parse()) {
                Some(Ok(m)) => m,
                _ => usize::MAX,
            };

            writeln!(builder, "Inspecting process information");

            let mut sys_info = System::new();

            sys_info.refresh_processes();

            let processes = sys_info.get_processes();

            let mut total_ram = 0usize;
            let mut total_count = 0u32;

            let processes = processes.values();

            let mut instance_violation = false;

            for p in processes {
                let cmd = p.cmd();
                if cmd.is_empty() {
                    continue;
                }
                let process_cmd = &cmd[0];

                let process_cmd_path = Path::new(&process_cmd);
                let cmd_name = match process_cmd_path
                    .file_name()
                    .map(|f| f.to_os_string().into_string())
                {
                    Some(Ok(name)) => name,
                    _ => continue,
                };

                writeln!(builder, "Found matching process with cmd: {}", cmd_name.trim());

                let is_match = if absolute_path {
                    executable_name == *process_cmd
                } else {
                    executable_name == cmd_name.trim()
                };

                if is_match {
                    let instance_ram = (p.memory() * 1024) as usize;

                    if instance_ram > max_ram_instance {
                        writeln!(builder, "Maximum RAM for any one process must be {} bytes or less. I got {} bytes", max_ram_instance, instance_ram);

                        instance_violation = true;
                    }

                    total_ram += instance_ram;
                    total_count += 1;
                }
            }

            writeln!(builder, "Found {} process(es) that match", total_count);
            writeln!(builder, "Checking if total process memory over limit");

            if instance_violation {
                return Ok(builder.down(
                    format!(
                        "All matching processes should use less than {} bytes of memory",
                        max_ram_instance
                    ),
                    "At least one process violates the memory limit",
                ));
            }

            writeln!(builder, "Checking if total process memory over limit");

            if total_ram > max_ram_total {
                writeln!(builder, "Maximum sum of RAM must be {} bytes or less. I got {} bytes", max_ram_total, total_ram);

                return Ok(builder.down(
                    format!(
                        "Total memory of matching processes should be less than {} bytes",
                        max_ram_total
                    ),
                    format!("Total memory of matching processes was {} bytes", total_ram),
                ));
            }

            writeln!(builder, "Checking that process count not over maximum count");

            if let Some(minimum_count) = monitor.body.minimum_count {
                writeln!(&mut builder, "Minimum number of processes is {}. I found {}", minimum_count, total_count);

                if total_count < minimum_count as u32 {
                    writeln!(&mut builder, "Failing because minimum proceess count not reached");

                    return Ok(builder.down(
                        format!(
                            "Should be at least {} process(es) that match",
                            minimum_count
                        ),
                        format!("Found {} process(es) that match", total_count),
                    ));
                }
            }

            writeln!(builder, "Checking that process count not below minimum count");

            if let Some(maximum_count) = monitor.body.maximum_count {
                writeln!(&mut builder, "Maximum number of processes is {}. I found {}", maximum_count, total_count);

                if maximum_count > total_count as isize {
                    writeln!(builder, "Failing because number of processes found is over limit");

                    return Ok(builder.down(
                        format!(
                            "Found {} or fewer matching process(es)",
                            maximum_count
                        ),
                        format!("Found {} process(es) that match", total_count),
                    ));
                }
            }

            writeln!(builder, "All OK");

            Ok(builder.ok(
                "All matching processes should be below threshold in monitor",
                "No process violated the memory or total count rules",
            ))
        })
    }
}
