//! QuantumTunnel Config

use serde::{Deserialize, Serialize};

/// QuantumTunnel Configuration
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct QuantumTunnelConfig {
    pub cosmos: CosmosConfig,
    pub substrate: SubstrateConfig,
}

// Cosmos Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CosmosConfig {
    pub chain_id: String,
    pub rpc_addr: String,
    pub seed: String, // temporary, not suitable for production use.
    //pub account_prefix: String, // do we need this?
    pub gas: u64,
    pub gas_price: String,
    pub default_denom: String,
    pub trusting_period: String,
    pub unbonding_period: String,
    pub wasm_id: u64,
}

// Default values for Cosmos Chain Configuration
impl Default for CosmosConfig {
    fn default() -> Self {
        Self {
            chain_id: "<chain_id>".to_owned(),
            rpc_addr: "http://localhost:26657/".to_owned(),
            seed: "twelve word private seed for the relayer acccount on the cosmos chain".to_owned(),
            gas: 500000,
            gas_price: "0.00025stake".to_owned(),
            default_denom: "stake".to_owned(),
            trusting_period: "144h".to_owned(),
            unbonding_period: "504h".to_owned(),
            wasm_id: 1,
        }
    }
}

// Substrate Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubstrateConfig {

    pub ws_addr: String,
    pub seed: String, // temporary, not suitable for production use.
    pub trusting_period: String,
    pub unbonding_period: String,
    pub max_clock_drift: String,

}

impl Default for SubstrateConfig {
    fn default() -> Self {
        Self {
            ws_addr: "ws://localhost:9944/".to_owned(),
            seed: "twelve word private seed for the relayer acccount on the cosmos chain".to_owned(),
            trusting_period: "144h".to_owned(),
            unbonding_period: "504h".to_owned(),
            max_clock_drift: "30s".to_owned(),
        }
    }
}
