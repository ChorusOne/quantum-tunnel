//! QuantumTunnel Config

use serde::{Deserialize, Serialize};

/// QuantumTunnel Configuration
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct QuantumTunnelConfig {
    /// Configurtion pertaining to the cosmos chain.
    pub cosmos: CosmosChainConfig,
    /// Configuration pertaining to the substrate chain.
    pub substrate: SubstrateChainConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CosmosChainConfig {
    #[serde(rename = "real")]
    Real(CosmosConfig),

    #[serde(rename = "simulation")]
    Simulation(String),
}

impl Default for CosmosChainConfig {
    fn default() -> Self {
        Self::Real(CosmosConfig::default())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SubstrateChainConfig {
    #[serde(rename = "real")]
    Real(SubstrateConfig),

    #[serde(rename = "simulation")]
    Simulation(String),
}

impl Default for SubstrateChainConfig {
    fn default() -> Self {
        Self::Real(SubstrateConfig::default())
    }
}

/// Cosmos Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CosmosConfig {
    /// cosmos chain id
    pub chain_id: String,
    /// address of cosmos websocket
    pub rpc_addr: String,
    /// address of cosmos rest service
    pub lcd_addr: String,
    /// seed of relayer account on cosmos chain. temporary, not suitable for production use. TODO: remove me.
    pub seed: String,
    //pub account_prefix: String, // do we need this?
    /// gas amount to send with transactions.
    pub gas: u64,
    /// price to pay per unit of gas.
    pub gas_price: String,
    /// default denomination on cosmos chain.
    pub default_denom: String,
    /// trusting period, e.g. 72h; must be less that unbonding_period
    pub trusting_period: String,
    /// unbonding period of chain, e.g. 504h
    pub unbonding_period: String,
    /// max clock drift tolerance
    pub max_clock_drift: String,
    /// identifier of the wasm blob uploaded into the wormhole module on cosmos chain.
    pub wasm_id: u32,
}

// Default values for Cosmos Chain Configuration
impl Default for CosmosConfig {
    fn default() -> Self {
        Self {
            chain_id: "<chain_id>".to_owned(),
            rpc_addr: "http://localhost:26657/".to_owned(),
            lcd_addr: "http://localhost:1317/".to_owned(),
            seed: "twelve word private seed for the relayer acccount on the cosmos chain"
                .to_owned(),
            gas: 500000,
            gas_price: "0.00025stake".to_owned(),
            default_denom: "stake".to_owned(),
            trusting_period: "144h".to_owned(),
            unbonding_period: "504h".to_owned(),
            max_clock_drift: "30s".to_owned(),
            wasm_id: 1,
        }
    }
}

/// Substrate Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubstrateConfig {
    /// address of websocket on substrate chain
    pub ws_addr: String,
    /// address of rpc socket on substrate chain
    pub rpc_addr: String,
    /// private seed of relayer on substrate side. subkey compatible, e.g. //Alice//hard; temporary, TODO: remove me
    pub seed: String,
    /// trusting period, e.g. 72h; must be less that unbonding_period
    pub trusting_period: String,
    /// unbonding period of chain, e.g. 504h
    pub unbonding_period: String,
    /// clock drift tolerance.
    pub max_clock_drift: String,
}

impl Default for SubstrateConfig {
    fn default() -> Self {
        Self {
            ws_addr: "ws://localhost:9944/".to_owned(),
            rpc_addr: "http://localhost:9933/".to_owned(),
            seed: "twelve word private seed for the relayer acccount on the cosmos chain"
                .to_owned(),
            trusting_period: "72h".into(),
            unbonding_period: "504h".into(),
            max_clock_drift: "30s".into(),
        }
    }
}
