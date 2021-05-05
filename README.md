[![Build Status][travis-badge]][travis]

[travis-badge]: https://travis-ci.org/ChorusOne/quantum-tunnel.svg?branch=master
[travis]: https://travis-ci.org/ChorusOne/quantum-tunnel/

# QuantumTunnel
QuantumTunnel is a basic relayer developed to connect Light Client ([tendermint_light_client], [substrate_light_client], [celo_light_client]) with cosmos chain. It can also be used
to test either substrate_light_client, tendermint_light_client or celo_light_client by simulating their target chains. Refer [here](#how-it-works) to know how it works under the hood.

This application is authored using [Abscissa], a Rust application framework.

## Compilation
To build QuantumTunnel you need to have rust nightly toolchain installed. You can find instructions [here](https://github.com/rust-lang/rustup#installing-nightly).
After you have nightly toolchain installed, just run:
`CHAIN=substrate make build`

## Running
QuantumTunnel relies on json files to read chain connection configuration, passed by `-c` command line argument.
QuantumTunnel can either connects to live chain or read simulation data from file and pass it to light client.

If we are connecting to live chain as opposed to read simulation data, you need to set following environment variables:

For Substrate live chain:
`SUBSTRATE_SIGNER_SEED=<your >= 12 words seed>`

For Cosmos live chain:
`COSMOS_SIGNER_SEED=<your >= 12 words seed>`

For Celo live chain:
`CELO_SIGNER_SEED=<your >= 12 words seed>`

The `test_data` folder in the repository contains different type of configuration and simulation data for both cosmos and substrate chain.
Each chain's configuration field in json can be of two forms: `real` or `simulation`. 
Let's take a look at an example configuration:
```json
{
  "cosmos": {
    "real": {
      "chain_id": "testing",
      "rpc_addr": "http://localhost:26657/",
      "lcd_addr": "http://localhost:1317/",
      "trusting_period": "720h",
      "unbonding_period": "721h",
      "max_clock_drift": "30s",
      "wasm_id": 1,
      "gas": 90000000,
      "gas_price": "0.25stake",
      "default_denom": "stake"
    }
  },
  "substrate": {
    "simulation": {
      "simulation_file_path": "test_data/substrate_light_client_simulated_2.txt",
      "should_run_till_height": 7
    }
  }
}
```

In this example, QuantumTunnel will connect to a *real* cosmos chain exposing rpc interface at port `26657`, but on substrate side it will read headers from the file `substrate_light_client_simulated_2.txt`.
This config implies to QuantumTunnel that we want to test `substrate_light_client` running on cosmos chain with simulation data contained in `substrate_light_client_simulated_2.txt` and the simulation will be considered success only if the `substrate_light_client` will run till height `7`. This feature is useful to test light client against invalid header sequence. If the simulation is successful quantum tunnel will exit with zero, otherwise it will panic and exit with non-zero status code.

## How it works?
Quantum tunnel is asynchronus application relies on [tokio] to handle four tasks, which communicates with each other using [crossbeam] channels:
1. Cosmos `send` handler: Receives substrate header data from Substrate receive handler and `send` them to substrate light client running inside the cosmos chain. 
2. Cosmos `receive` handler: `Receives` new headers from cosmos blockchain or simulation file and send them to Substrate send handler.
3. Substrate `send` handler: Receives new cosmos headers from Cosmos receive handler and `send` them to cosmos light client running inside the substrate chain.
4. Substrate `receive` handler: `Receives` new headers from substrate blockchain or simulation file and send them to Cosmos send handler.

Each side's handlers can start in one of the two modes: 
- Simulation mode: In simulation mode, `receive` handler reads header data from file instead of querying live chain and also keeps track of how many blocks has been consumed by opposite chain's `send` handler to determine result of the simulation. `send` handler in simulation mode just drains header data sent by opposite chain's `receive` handler, to prevent crossbeam channel to accumulate large number of unsent data. At max only one chain's handlers can run in `simulation` mode.
- Real mode: In real mode, `receive` handler reads header data from live chain and pass it to opposite chain's `send` handler, which formats it and sends it to light client running in its chain.

[Abscissa]: https://github.com/iqlusioninc/abscissa
[tendermint_light_client]: https://github.com/ChorusOne/tendermint-light-client
[substrate_light_client]: https://github.com/ChorusOne/substrate-light-client
[celo_light_client]: https://github.com/ChorusOne/celo-light-client
[tokio]: https://github.com/tokio-rs/tokio
[crossbeam]: https://github.com/crossbeam-rs/crossbeam
