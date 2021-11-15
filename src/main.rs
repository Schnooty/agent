//#![deny(warnings)]
extern crate chrono;
extern crate clap;
extern crate env_logger;
extern crate futures;
extern crate openapi_client;
extern crate serde_json;
extern crate sysinfo;
extern crate toml;
#[macro_use]
extern crate log;
extern crate actix;
extern crate actix_rt;
extern crate rand;
#[cfg(test)]
extern crate test_logger;
extern crate num_cpus;
extern crate hostname;
extern crate lazy_static;
extern crate native_tls;
extern crate lettre_email;
extern crate redis;
extern crate serde_yaml;
extern crate async_std;
extern crate http as http_types;
extern crate async_native_tls;
extern crate base64;

mod error;
mod monitoring;
mod actors;
mod api;
mod alerts;
mod config;
mod http;

use clap::{AppSettings, Clap};
use std::time::Duration;
use crate::actix::Actor;
use crate::actors::*;
use crate::api::HttpApi;
use crate::config::*;
use std::fs::File;
use std::io::Read;


#[derive(Clap, Debug)]
#[clap(version = "0.1.1", author = "Mate Antunovic <mate AT schnooty.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Relative path to the agent config file (in TOML) format.
    #[clap(long)]
    config: String,
}

#[actix_rt::main]
async fn main() {
    println!("Starting Schnooty Agent");

    if !env_logger::init().is_ok() {
        println!("Failed to initialise the logger. Stopping");
        std::process::exit(1);
    }

    let opts: Opts = Opts::parse();
    let config_file_path = &opts.config;
    info!("Loading config from: {}", config_file_path);

    let mut file = match File::open(config_file_path) {
        Ok(f) => f,
        Err(err) => {
            error!("Error loading config from {}: {}", config_file_path, err);
            std::process::exit(1);
        }
    };

    let mut contents = String::new();

    info!("Parsing config");

    match file.read_to_string(&mut contents) {
        Ok(_) => {},
        Err(err) => {
            error!("Failed to load file at {}: {}", config_file_path, err);
            return;
        }
    };

    let config: Config = match serde_yaml::from_str(&contents) {
        Ok(c) => c,
        Err(err) => {
            error!("Failed to parse config file at {}: {}", config_file_path, err);
            return;
        }
    };

    let api_addr = match &config.base_url {
        Some(ref url) => {
            debug!("Using {} as base URL", url);
            let api = HttpApi::new(&api::HttpConfig {
                base_url: url.clone(),
                api_key: config.api_key.clone()
            });

            let api_actor = actors::ApiActor::new(api);
            debug!("Starting the API actor");
            Some(api_actor.start())
        },
        None => {
            debug!("No base URL supplied. Not going to use API");
            None
        }
    };

    debug!("Starting the Actix system");

    let alerter = actors::AlerterActor::new(alerts::AlertApiImpl::new());
    let alerter_addr = alerter.start();

    let status_recipients = if let Some(ref api_addr) = &api_addr {
        let uploader = actors::UploaderActor::new(api_addr.clone());
        let uploader_addr = uploader.start();

        vec![
            uploader_addr.recipient(),
            alerter_addr.clone().recipient()
        ]
    } else {
        info!("No API URL. Statuses will not be uploaded");
        vec![]
    };

    let monitoring = monitoring::MonitorFutureMaker::new();

    let executor_actor = actors::ExecutorActor::new(monitoring, status_recipients);
    let executor_addr = executor_actor.start();

    let scheduler_actor = actors::SchedulerActor::new(vec![
        executor_addr.recipient()
    ]);
    let scheduler_addr = scheduler_actor.start();

    // all the configurators

    let configurator = ConfiguratorActor::new(
        vec![scheduler_addr.recipient()],
        vec![alerter_addr.recipient()],
    );

    let session_actor = actors::SessionActor::new(
        vec![]
    );
    let session_actor_addr = session_actor.start();
    let configurator_addr = configurator.start();

    debug!("Starting the agent with actix");

    match session_actor_addr.send(CurrentConfig { config: config.clone() }).await {
        Ok(Ok(_)) => { 
            info!("Successfully started session");
        },
        Ok(Err(err)) => {
            info!("Error starting session: {}", err);
            std::process::exit(1);
        },
        Err(err) => {
            info!("Error starting session: {}", err);
            std::process::exit(1);
        }
    };

    match configurator_addr.send(CurrentConfig { config: config.clone() }).await {
        Ok(Ok(_)) => { 
            info!("Successfully started session");
        },
        Ok(Err(err)) => {
            info!("Error starting session: {}", err);
            std::process::exit(1);
        },
        Err(err) => {
            info!("Error starting session: {}", err);
            std::process::exit(1);
        }
    };

    debug!("Done in the main thread");
    loop {
         async_std::task::sleep(Duration::new(u64::MAX >> 32, 0)).await;
    }
}
