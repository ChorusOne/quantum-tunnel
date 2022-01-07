use crate::config::CosmosConfig;
use crate::utils::prost_serialize;
use crate::error::ErrorKind;
use crate::cosmos::types::{StdMsg, MsgCreateWasmClient, MsgUpdateWasmClient, WasmHeader};
use crate::cosmos::proto::{
    ibc::core::commitment::v1::MerkleRoot,
    ibc::lightclients::wasm::v1::{ClientState, ConsensusState, Header as IbcWasmHeader},
    ibc::core::client::v1::{MsgCreateClient, MsgUpdateClient, Height},
};
use celo_types::header::Header as CeloHeader;
use celo_types::{client::LightClientState, consensus::LightConsensusState};
use serde::{Deserialize, Serialize};
use num::cast::ToPrimitive;
use prost_types::Any;
use std::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CeloWrappedHeader {
    pub header: CeloHeader,
    pub initial_consensus_state: LightConsensusState,
    pub initial_client_state: LightClientState,
}

impl WasmHeader for CeloWrappedHeader {
    fn chain_name() -> &'static str {
        "Celo"
    }

    fn height(&self) -> u64 {
        self.header.number.as_u64()
    }

    fn to_wasm_create_msg(&self, cfg: &CosmosConfig, address: String) -> Result<Vec<Any>, Box<dyn Error>> {
        if self.header.number.as_u64() != self.initial_consensus_state.number {
            return Err(Box::new(ErrorKind::Io("initial block header doesn't match initial state entry height".to_string())));
        }

        let code_id = hex::decode(&cfg.wasm_id)?;
        let client_state = ClientState {
            code_id: code_id.clone(),
            data: rlp::encode(&self.initial_client_state).as_ref().to_vec(),
            frozen: false,
            frozen_height: None,
            latest_height: Some(Height {
                revision_number: 0,
                revision_height: self.header.number.as_u64(),
            }),
            r#type: "wasm_dummy".to_string(),
        };

        let consensus_state = ConsensusState {
            code_id,
            data: rlp::encode(&self.initial_consensus_state).as_ref().to_vec(),
            timestamp: self.header.time.as_u64(),
            root: Some(MerkleRoot { hash: self.header.root.as_bytes().to_vec() }),
            r#type: "wasm_dummy".to_string()
        };

        let msg = MsgCreateClient {
            client_state: Some(Any {
                type_url: "/ibc.lightclients.wasm.v1.ClientState".to_string(),
                value: prost_serialize(&client_state)?,
            }),
            consensus_state: Some(Any {
                type_url: "/ibc.lightclients.wasm.v1.ConsensusState".to_string(),
                value: prost_serialize(&consensus_state)?,
            }),
            signer: address,
        };

        Ok(vec![
            Any {
                type_url: MsgCreateWasmClient::<Self>::get_type(),
                value: prost_serialize(&msg)?,
            }
        ])
    }

    fn to_wasm_update_msg(&self, address: String, client_id: String) -> Result<Vec<Any>, Box<dyn Error>> {
        let header = IbcWasmHeader {
            data: rlp::encode(&self.header).as_ref().to_vec(),
            height: Some(Height {
                revision_number: 0,
                revision_height: self.header.number.as_u64(),
            }),
            r#type: "wasm_dummy".to_string()
        };

        let msg = MsgUpdateClient {
            client_id,
            header: Some(Any {
                type_url: "/ibc.lightclients.wasm.v1.Header".to_string(),
                value: prost_serialize(&header)?,
            }),
            signer: address,
        };

        Ok(vec![
            Any {
                type_url: MsgUpdateWasmClient::<Self>::get_type(),
                value: prost_serialize(&msg)?,
            }
        ])
    }
}
