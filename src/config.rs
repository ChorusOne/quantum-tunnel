//! QuantumTunnel Config

use serde::{Deserialize, Serialize};

/// QuantumTunnel Configuration
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct QuantumTunnelConfig {
    /// Configuration pertaining to the cosmos chain.
    pub cosmos: CosmosChainConfig,
    /// Configuration pertaining to the substrate chain.
    pub substrate: Option<SubstrateChainConfig>,
    /// Configuration pertaining to the celo chain.
    pub celo: Option<CeloChainConfig>,
}

/// Cosmos chain specific configuration enum
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CosmosChainConfig {
    /// Quantum tunnel will try to connect to live channel
    #[serde(rename = "real")]
    Real(CosmosConfig),

    /// Quantum tunnel will read from target pointed
    /// by `CosmosSimulationConfig`
    #[serde(rename = "simulation")]
    Simulation(CosmosSimulationConfig),
}

impl Default for CosmosChainConfig {
    fn default() -> Self {
        Self::Real(CosmosConfig::default())
    }
}

/// Substrate chain specific configuration enum
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SubstrateChainConfig {
    /// Quantum tunnel will try to connect to live channel
    #[serde(rename = "real")]
    Real(SubstrateConfig),

    /// Quantum tunnel will read from target pointed
    /// by `SubstrateSimulationConfig`
    #[serde(rename = "simulation")]
    Simulation(SubstrateSimulationConfig),
}

impl Default for SubstrateChainConfig {
    fn default() -> Self {
        Self::Real(SubstrateConfig::default())
    }
}

/// Celo chain specific configuration enum
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CeloChainConfig {
    /// Quantum tunnel will try to connect to live channel
    #[serde(rename = "real")]
    Real(CeloConfig),

    /// Quantum tunnel will read from target pointed
    /// by `CeloSimulationConfig`
    #[serde(rename = "simulation")]
    Simulation(CeloSimulationConfig),
}

impl Default for CeloChainConfig {
    fn default() -> Self {
        Self::Real(CeloConfig::default())
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
    pub grpc_addr: String,
    /// Bip39 seed of relayer account on cosmos chain. Does not serialize/deserialize.
    #[serde(skip)]
    pub signer_seed: String,
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
    pub wasm_id: String,
    /// Flag indicating whether opposite side is simulation. Does not serialize/deserialize.
    #[serde(skip)]
    pub is_other_side_simulation: bool,
}

// Default values for Cosmos Chain Configuration
impl Default for CosmosConfig {
    fn default() -> Self {
        Self {
            chain_id: "<chain_id>".to_owned(),
            rpc_addr: "http://localhost:26657/".to_owned(),
            grpc_addr: "http://localhost:9090".to_owned(),
            signer_seed: "".to_owned(),
            gas: 500000,
            gas_price: "0.00025stake".to_owned(),
            default_denom: "stake".to_owned(),
            trusting_period: "144h".to_owned(),
            unbonding_period: "504h".to_owned(),
            max_clock_drift: "30s".to_owned(),
            wasm_id: "<wasm_id>".to_string(),
            is_other_side_simulation: false,
        }
    }
}

/// Cosmos Chain Simulation Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CosmosSimulationConfig {
    /// Path of the simulation file
    pub simulation_file_path: String,
    /// Simulation run till this specific height
    pub should_run_till_height: u64,
}

/// Substrate Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubstrateConfig {
    /// address of websocket on substrate chain
    pub ws_addr: String,
    /// address of rpc socket on substrate chain
    pub rpc_addr: String,
    /// Bip39 seed of relayer account on substrate chain. Does not serialize/deserialize.
    #[serde(skip)]
    pub signer_seed: String,
    /// trusting period, e.g. 72h; must be less that unbonding_period
    pub trusting_period: String,
    /// unbonding period of chain, e.g. 504h
    pub unbonding_period: String,
    /// clock drift tolerance.
    pub max_clock_drift: String,
    /// Flag indicating whether opposite side is simulation or not. Does not serialize/deserialize.
    #[serde(skip)]
    pub is_other_side_simulation: bool,
}

impl Default for SubstrateConfig {
    fn default() -> Self {
        Self {
            ws_addr: "ws://localhost:9944/".to_owned(),
            rpc_addr: "http://localhost:9933/".to_owned(),
            signer_seed: "".to_owned(),
            trusting_period: "72h".into(),
            unbonding_period: "504h".into(),
            max_clock_drift: "30s".into(),
            is_other_side_simulation: false,
        }
    }
}

/// Substrate Chain Simulation Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubstrateSimulationConfig {
    /// Path of the simulation file
    pub simulation_file_path: String,
    /// Simulation should run till this specific height
    /// to be considered successful.
    pub should_run_till_height: u64,
}

/// Celo Chain Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CeloConfig {
    /// address of websocket on celo chain
    pub ws_addr: String,
    /// address of geth rpc on celo chain
    pub rpc_addr: String,
    /// the size of epoch on celo chain
    pub epoch_size: u64,
    /// Bip39 seed of relayer account on cosmos chain. Does not serialize/deserialize.
    #[serde(skip)]
    pub signer_seed: String,
    /// flag indicating whether epoch headers should be validated against BLS signature
    pub verify_epoch_headers: bool,
    /// flag indicating whether non-epoch headers should be validated against BLS signature
    pub verify_non_epoch_headers: bool,
    /// flag indicating whether timestamp validation should be enabled (this includes max_clock_drift verification)
    pub verify_header_timestamp: bool,
    /// flag indicating whether expired LC can be updated
    pub allow_update_after_expiry: bool,
    /// flag indicating whether frozen LC due to misbehaviour can be updated
    pub allow_update_after_misbehavior: bool,
    /// trusting period, e.g. 72h
    pub trusting_period: String,
    /// max clock drift tolerance
    pub max_clock_drift: String,
    /// Flag indicating whether opposite side is simulation. Does not serialize/deserialize.
    #[serde(skip)]
    pub is_other_side_simulation: bool,
}

// Default values for Cosmos Chain Configuration
impl Default for CeloConfig {
    fn default() -> Self {
        Self {
            ws_addr: "ws://localhost:3334/".to_owned(),
            rpc_addr: "http://localhost:8545".to_owned(),
            epoch_size: 17280,
            signer_seed: "".to_owned(),
            verify_epoch_headers: true,
            verify_non_epoch_headers: true,
            verify_header_timestamp: true,
            allow_update_after_expiry: true,
            allow_update_after_misbehavior: true,
            trusting_period: "144h".to_owned(),
            max_clock_drift: "30s".to_owned(),
            is_other_side_simulation: false,
        }
    }
}

/// Celo Chain Simulation Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CeloSimulationConfig {
    /// Path of the simulation file
    pub simulation_file_path: String,
    /// Simulation run till this specific height
    pub should_run_till_height: u64,
}
