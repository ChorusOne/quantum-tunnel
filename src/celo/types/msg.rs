use crate::config::CosmosConfig;
use crate::utils::prost_serialize;
use crate::error::ErrorKind;
use crate::cosmos::types::{StdMsg, MsgCreateWasmClient, MsgUpdateWasmClient, WasmHeader};
use crate::cosmos::proto::{
    ibc::core::commitment::v1::MerkleRoot,
    ibc::lightclients::wasm::v1::{ClientState, ConsensusState, Header as IbcWasmHeader},
    ibc::core::client::v1::{MsgCreateClient, MsgUpdateClient, Height},
};
use celo_light_client::{
    Header as CeloHeader,
    ToRlp,
    contract::types::state::{
        LightConsensusState,
        LightClientState
    },
};
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
        self.header.number.to_u64().unwrap()
    }

    fn to_wasm_create_msg(&self, cfg: &CosmosConfig, address: String) -> Result<Vec<Any>, Box<dyn Error>> {
        if self.header.number.to_u64().unwrap() != self.initial_consensus_state.number {
            return Err(Box::new(ErrorKind::Io("initial block header doesn't match initial state entry height".to_string())));
        }

        let code_id = hex::decode(&cfg.wasm_id)?;
        let client_state = ClientState {
            code_id: code_id.clone(),
            data: self.initial_client_state.to_rlp(),
            latest_height: Some(Height {
                revision_number: 0,
                revision_height: self.header.number.to_u64().unwrap(),
            }),
            proof_specs: Vec::new(),
        };

        let consensus_state = ConsensusState {
            code_id,
            data: self.initial_consensus_state.to_rlp(),
            timestamp: self.initial_consensus_state.timestamp,
            root: Some(MerkleRoot { hash: self.header.root.to_vec() }),
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
            data: self.header.to_rlp().to_owned(),
            height: Some(Height {
                revision_number: 0,
                revision_height: self.header.number.to_u64().unwrap(),
            }),
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
