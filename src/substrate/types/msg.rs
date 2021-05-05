use crate::config::CosmosConfig;
use crate::cosmos::types::{MsgCreateWasmClient, MsgUpdateWasmClient, StdMsg, WasmHeader};
use crate::substrate::types::{CreateSignedBlockWithAuthoritySet, SignedBlockWithAuthoritySet};
use parse_duration::parse;
use serde_json::Value;
use std::error::Error;

impl WasmHeader for SignedBlockWithAuthoritySet {
    fn chain_name() -> &'static str {
        "Substrate"
    }

    fn height(&self) -> u64 {
        self.block.block.header.number as u64
    }

    fn to_wasm_create_msg(
        &self,
        cfg: &CosmosConfig,
        address: String,
        client_id: String,
    ) -> Result<Vec<Value>, Box<dyn Error>> {
        let msg = MsgCreateWasmClient {
            header: CreateSignedBlockWithAuthoritySet {
                block: self.block.clone(),
                authority_set: self.authority_set.clone(),
                set_id: self.set_id,
                max_headers_allowed_to_store: 256,
                max_headers_allowed_between_justifications: 512,
            },
            address,
            trusting_period: parse(&cfg.trusting_period)?.as_nanos().to_string(),
            max_clock_drift: parse(&cfg.max_clock_drift)?.as_nanos().to_string(),
            unbonding_period: parse(&cfg.unbonding_period)?.as_nanos().to_string(),
            client_id,
            wasm_id: cfg.wasm_id,
        };

        Ok(vec![
            serde_json::json!({"type": MsgCreateWasmClient::<SignedBlockWithAuthoritySet>::get_type(), "value": &msg}),
        ])
    }

    fn to_wasm_update_msg(&self, address: String, client_id: String) -> Vec<Value> {
        let msg = MsgUpdateWasmClient {
            header: self,
            address,
            client_id,
        };

        vec![
            serde_json::json!({"type": MsgUpdateWasmClient::<SignedBlockWithAuthoritySet>::get_type(), "value": &msg}),
        ]
    }
}
