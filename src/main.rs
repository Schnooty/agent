#![deny(warnings)]
extern crate chrono;
extern crate clap;
extern crate env_logger;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate openapi_client;
extern crate reqwest;
extern crate serde_json;
extern crate sysinfo;
extern crate tokio_timer;
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
extern crate async_std;
extern crate redis;

mod error;
mod monitoring;
mod actors;
mod api;
mod alerts;
mod config;

use clap::{AppSettings, Clap};
use actix::clock::Duration;
use actix::clock::delay_for;
use crate::actix::Actor;
use crate::actors::SessionInit;
use crate::api::HttpApi;
use crate::config::Config;
use std::fs::File;
use std::io::Read;

#[derive(Clap, Debug)]
#[clap(version = "0.1.1", author = "Mate Antunovic <mate AT schnooty.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Relative path to the agent config file (in TOML) format.
    #[clap(long)]
    config: Option<String>,
    /// Agent API key which will be used to authenticate with the Schnooty API.
    #[clap(long)]
    api_key: Option<String>,
    /// Base URL of Schnooty API or a custom API (for overriding).
    #[clap(long)]
    base_url: Option<String>,
    #[clap(long)]
    /// Override the default 'main' group ID. Agents in with the group ID divide monitors between themselves.
    group_id: Option<String>,
    #[clap(long)]
    /// Monitor file
    monitors_file: Option<String>,
    #[clap(long)]
    /// Agents file
    alerts_file: Option<String>
}

#[actix_rt::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let mut base_config: Config = if let Some(ref config_file_path) = &opts.config {
        println!("Loading config from {}", config_file_path);

        let mut file = match File::open(&config_file_path) {
            Ok(f) => f,
            Err(err) => {
                println!("Error loading config from {}: {}", config_file_path, err);
                return;
            }
        };
        let mut contents = String::new();

        match file.read_to_string(&mut contents) {
            Ok(_) => {},
            Err(err) => {
                println!("Failed to load file at {}: {}", config_file_path, err);
                return;
            }
        };
    
        match toml::from_str(&contents) {
            Ok(c) => c,
            Err(err) => {
                println!("Failed to parse config file at {}: {}", config_file_path, err);
                return;
            }
        }
    } else {
        Default::default()
    };

    base_config.base_url = if opts.base_url.is_some() { opts.base_url } else { base_config.base_url };
    base_config.api_key = if opts.api_key.is_some() { opts.api_key } else { base_config.api_key };
    base_config.group_id = opts.group_id.unwrap_or(base_config.group_id);
    base_config.monitor_file = if opts.monitors_file.is_some() { opts.monitors_file } else { None };
    base_config.alert_file = if opts.alerts_file.is_some() { opts.alerts_file } else { None };

    if base_config.api_key.is_none() {
        println!("You have started Schnooty without an API key. This is required to communicate with Schnooty API.");
        println!("Supply one with the '--api-key' option or use a config TOML file with '--config'");
    }

    let config = base_config;

    let api_addr = match config.base_url {
        Some(url) => {
            debug!("Using {} as base URL", url);
            let api = HttpApi::new(&api::HttpConfig {
                base_url: url,
                api_key: config.api_key.clone()
            });

            let api_actor = actors::ApiActor::new(api);
            Some(api_actor.start())
        },
        None => {
            debug!("No base URL supplied. Not going to use API");
            None
        }
    };

    //let base_url = config.base_url.clone().parse::<hyper::Uri>().unwrap();

    //info!("Base URL is: {}", base_url);

    let group_id = config.group_id;
    //let api_key = config.api_key;

    let agent_id: String = if let Some(ref api_key) = &config.api_key {
        match api_key.split(':').next() {
            Some(ref a) => a.to_owned().to_string(),
            _ => { 
                error!("Invalid API key");
                return;
            }
        }
    } else {
        "anonymous-agent".to_owned()
    };


    println!("Starting Schnooty Agent");

    if !env_logger::init().is_ok() {
        println!("Failed to initialise the logger. Stopping");
    }

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

    let scheduler_actor = actors::SchedulerActor::new(executor_addr.recipient());
    let scheduler_addr = scheduler_actor.start();

    let monitor_file_addr = match config.monitor_file {
        Some(p) => Some(actors::FileActor::new(p).start()),
        None => None
    };

    let configurator_actor = actors::ConfiguratorActor::new(
        api_addr.clone(),
        scheduler_addr,
        vec![alerter_addr.recipient()],
        monitor_file_addr
    );
    let configurator_addr = configurator_actor.start();

    let session_actor =
        actors::SessionActor::new(&agent_id, &group_id, api_addr.clone(), configurator_addr);
    let session_actor_addr = session_actor.start();

    debug!("Running the Actix system");

    loop {
        match session_actor_addr.send(SessionInit {}).await {
            Ok(Ok(_)) => { 
                info!("Successfully started session");
                break;
            },
            Ok(Err(err)) => info!("Error starting session{}", match err.error { Some(ref err) => format!(": {}", err.to_string()), None => String::new() }),
            Err(err) => error!("Error starting session: {}", err),
        };

        const FIVE_SECONDS: u64 = 5;

        info!("Will try to start session again in {} second(s)", FIVE_SECONDS);

        delay_for(Duration::new(FIVE_SECONDS, 0)).await;
    }

    loop {
        async { delay_for(Duration::new(u16::MAX as u64, 0)).await }.await;
    }
}
