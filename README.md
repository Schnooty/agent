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

Schnooty Agent on your Linux distribution. You can also compile and run from source. Support for Windows binaries is slated.

<!-- * ex. Windows 10 -->

The agent connects to `api.schnooty.com` to load the latest configuration.

### Installing

Download the latest binaries on GitHub from [the releases page](https://github.com/Schnooty/agent/releases).

Schnooty is released only as a standalone binary. You can configure as a daemon, system V service, or Windows service. 


### Executing program

Ensure that you set up the `api_key` property in `config.toml`. You need this for the agent to load its set up 
from `api.schnooty.com`.

```
cat 'api_key="YOUR_API_KEY_HERE"' > config.toml
```

Ensure that `config.toml` is in the same directory as the `schnooty` executable. Then run it

```
./schnooty
```

The program will run forever, which makes it suitable as a daemon or background process.

## Authors and contributors

The Schnooty Agent is produced by Schnooty. [@synlestidae](https://github.com/synlestidae) is the current author.

Authors and contributors will be listed here as contributions are received.

## Version History

* 0.0.1
    * The first version of the agent is still under active development and subject to change.

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT) - see the [LICENSE](LICENCE) for details.

## Links

Inspiration, code snippets, etc.
* [Schnooty homepage](https://www.schnooty.com)
* [Rust programming language](https://www.rust-lang.org/)
