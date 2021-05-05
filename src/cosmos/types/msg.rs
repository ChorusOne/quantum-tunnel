use crate::config::CosmosConfig;
use serde::{Deserialize, Serialize};
use std::error::Error;
use prost_types::Any;

pub trait StdMsg {
    fn get_type() -> String
    where
        Self: Sized;
}

/// Payload to initialize substrate light client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgCreateWasmClient<T> {
    pub client_id: String,
    pub header: T,
    pub trusting_period: String,
    pub unbonding_period: String,
    pub max_clock_drift: String,
    pub address: String,
    #[serde(with = "crate::utils::from_str")]
    pub wasm_id: u32,
}

pub trait WasmHeader {
    fn chain_name() -> &'static str;
    fn height(&self) -> u64;

    fn to_wasm_create_msg(&self, cfg: &CosmosConfig, address: String) -> Result<Vec<Any>, Box<dyn Error>>;
    fn to_wasm_update_msg(&self, address: String, client_id: String) -> Result<Vec<Any>, Box<dyn Error>>;
}

impl<T> StdMsg for MsgCreateWasmClient<T> {
    fn get_type() -> String {
        "/ibc.core.client.v1.MsgCreateClient".to_owned()
    }
}

/// Payload to update substrate light client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgUpdateWasmClient<T>{
    pub client_id: String,
    pub header: T,
    pub address: String,
}

impl<T> StdMsg for MsgUpdateWasmClient<T> {
    fn get_type() -> String {
        "/ibc.core.client.v1.MsgUpdateClient".to_owned()
    }
}
