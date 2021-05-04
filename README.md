### 
# Schnooty Agent

Monitor everything using Schnooty Agent. Once you set things up on [schnooty.com](https://schnooty.com), you can get started
monitoring your servers, databases, and infrastructure.

## Description

Schnooty Agent runs in the background and monitors whatever you tell it to. The agent automatically keeps itself updated itself with the monitors and alerts you
set up. 

## Getting Started

Before running the agent, you will need a [Schnooty account](https://www.schnooty.com). You must set up an API key for the agent you are running.
Then you add some monitors and alerts which the agent will pick up and run.


### Dependencies

Schnooty Agent has binaries for Linux. You can also compile and run from source. Support for Windows binaries is set for a future date.

Linux
* OpenSSL v1.1.1 or greater.
* Access to `api.schnooty.com` over port 443 (proxies not supported).

### Installing

Download the latest binaries on GitHub from [the releases page](https://github.com/Schnooty/agent/releases).

Schnooty is released only as a standalone binary. You can configure it as [System V service](https://www.digitalocean.com/community/tutorials/how-to-configure-a-linux-service-to-start-automatically-after-a-crash-or-reboot-part-1-practical-examples) or Windows service.


### Executing program

Ensure that you set up the `api_key` property in `config.toml`. You need this for the agent to load its set up 
from `api.schnooty.com`. You also need `group_id`. Agents in the same group will divide monitors between themselves, ensuring that
no two agents share the same monitor. This feature will be removed in a future version of the agent and Schnooty API.

```
api_key = "YOUR_API_KEY"
group_id = "main"
```

Ensure that `config.toml` is in the same directory as the `schnooty` executable. 

```
$ ls 
schnooty config.toml
```

Choose a log level for the log output you want: error warn info debug trace. The agent uses Rust env-logger
and reads the level from `RUST_LOG`. 

```
RUST_LOG=debug ./schnooty
```

When you start it, the agent will run forever (unless it crashes), which 
makes it suitable as a daemon or background process.

```
INFO:<unknown>: Starting the monitor agent
INFO:<unknown>: Loading config from config.toml
DEBUG:<unknown>: Loaded config successfully. Parsing contents
INFO:<unknown>: Base URL is: https://api.schnooty.com/
DEBUG:<unknown>: Starting the Actix system
DEBUG:<unknown>: Running the Actix system
INFO:<unknown>: Starting a new session
```
## Authors

The Schnooty Agent is produced by Schnooty. [@synlestidae](https://github.com/synlestidae) is the current author.

Authors and contributors will be listed here as contributions are received.

## Version History

* 0.0.1
    * This version is under active development. Both the Schnooty agent and API are subject to change.

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT) - see the [LICENSE](LICENCE) for details.

## Links

* [Schnooty homepage](https://www.schnooty.com)
* [Rust programming language](https://rust-lang.org)
