#![deny(warnings)]
extern crate chrono;
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

mod error;
mod monitoring;
mod actors;
mod api;
mod alerts;
mod config;

use actix::clock::Duration;
use actix::clock::delay_for;
use crate::actix::Actor;
use crate::actors::SessionInit;
use crate::api::HttpApi;
use crate::config::Config;
use std::fs::File;
use std::io::Read;

#[actix_rt::main]
async fn main() {
    if !env_logger::init().is_ok() {
        println!("Failed to initialise the logged. Stopping");
    }

    let config_file_path = "config.toml";

    info!("Starting the monitor agent");
    info!("Loading config from {}", config_file_path);

    let mut file = File::open(config_file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    debug!("Loaded config successfully. Parsing contents");

    let config: Config = toml::from_str(&contents).unwrap();
    let http_api = HttpApi::new(&config);
    let base_uri = config.base_uri.clone().parse::<hyper::Uri>().unwrap();

    info!("Base URL is: {}", base_uri);

    let group_id = config.group_id;
    let api_key = config.api_key;

    let agent_id = match api_key.split(':').next() {
        Some(ref a) => a.to_owned(),
        _ => { 
            error!("Invalid API key: {}", api_key);
            return;
        }
    };

    debug!("Starting the Actix system");

    let api_actor = actors::ApiActor::new(http_api);
    let api_addr = api_actor.start();

    let uploader = actors::UploaderActor::new(api_addr.clone());
    let uploader_addr = uploader.start();

    let alerter = actors::AlerterActor::new(alerts::AlertApiImpl::new());
    let alerter_addr = alerter.start();

    let status_recipients = vec![
        uploader_addr.recipient(),
        alerter_addr.clone().recipient()
    ];

    let monitoring = monitoring::MonitorFutureMaker::new();

    let executor_actor = actors::ExecutorActor::new(monitoring, status_recipients);
    let executor_addr = executor_actor.start();

    let scheduler_actor = actors::SchedulerActor::new(executor_addr.recipient());
    let scheduler_addr = scheduler_actor.start();

    let configurator_actor = actors::ConfiguratorActor::new(api_addr.clone(), scheduler_addr, vec![alerter_addr.recipient()]);
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
