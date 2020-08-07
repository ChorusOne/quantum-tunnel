# QuantumTunnel
QuantumTunnel is a basic relayer developed to connect [tendermint_light_client] and [substrate_light_client] with any cosmos and substrate chain respectively. It can also be used
to test either substrate_light_client or tendermint_light_client by simulating their target chains.

This application is authored using [Abscissa], a Rust application framework.

## Compilation
To build QuantumTunnel you need to have rust nightly toolchain installed. You can find instructions [here](https://github.com/rust-lang/rustup#installing-nightly).
After you have nightly toolchain installed, just run:
`cargo +nightly build`

## Running
QuantumTunnel relies on json files to read chain connection configuration, passed by `-c` command line argument.
QuantumTunnel can either connects to live chain or read simulation data from file and pass it to light client.

If we are connecting to live chain as opposed to read simulation data, you need to set following environment variables:

For Substrate live chain:
`SUBSTRATE_SIGNER_SEED=<your >= 12 words seed>`

For Cosmos live chain:
`COSMOS_SIGNER_SEED=<your >= 12 words seed>`

The `test_data` folder in the repository contains different type of configuration and simulation data for both cosmos and substrate chain.
Each chain's configuration field in json can be of two forms: real or simulation. 
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
      "simulation": "substrate_light_client_simulated_2.txt"
  }
}
```

In this example, QuantumTunnel will connect to a *real* cosmos chain exposing rpc interface at port `26657`, but on substrate side it will read headers from the file `substrate_light_client_simulated_2.txt`.
This config implies to QuantumTunnel that we want to test `substrate_light_client` running on cosmos chain with simulation data contained in `substrate_light_client_simulated_2.txt`. This feature is useful to test light client against invalid header sequence.


[Abscissa]: https://github.com/iqlusioninc/abscissa
[tendermint_light_client]: https://github.com/ChorusOne/tendermint-light-client
[substrate_light_client]: https://github.com/ChorusOne/substrate-light-client
