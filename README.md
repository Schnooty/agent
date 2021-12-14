### 
# Schnooty Agent

Monitor everything using Schnooty Agent. Schnooty runs in the background and monitors whatever you tell it to. For example, your HTTP server. If it detects a problem, Schnooty can send you an alert. For example, in an email. Schnooty Agent is for developers who want efficient monitoring that is easy to set up.

## Getting Started

If you want to view your monitoring activity at [schnooty.com](https://www.schnooty.com), you will need to set up an account. Set up an API key for your agent. Then add your alerts to the agent's `config.yaml`.

### Dependencies

Schnooty Agent has binaries for Linux. You can also compile and run from source. Support for Windows binaries is not currently available.

Linux
* OpenSSL v1.1.1 or greater.
* Access to `api.schnooty.com` over port 443 (proxies not supported).

### Installing

Download the latest binaries on GitHub from [the releases page](https://github.com/Schnooty/agent/releases).

Schnooty is released only as a standalone binary. You can configure it as [System V service](https://www.digitalocean.com/community/tutorials/how-to-configure-a-linux-service-to-start-automatically-after-a-crash-or-reboot-part-1-practical-examples) or Windows service.

### Running it

Ensure that you set up the `api_key` property in `config.yaml`. You need this for the agent to connect to the Schnooty API. 

```
base_url: "http://localhost:3001/"
api_key: 90aa9eb6bfad4512959c854922f669e7:2314KyCx8CytA2vKsfR8vGfAl7WBfa
monitors: 
  - name: website-monitor
    type: http
    enabled: true
    period: 1m
    timeout: 5s
    body:
      url: https://www.mywebsite.com
status:
  enabled: true
session:
  name: test-name-session
  enabled: true
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
