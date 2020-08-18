use crate::config::{SubstrateChainConfig, SubstrateConfig};
use crate::cosmos::types::{TMCreateClientPayload, TMHeader, TMUpdateClientPayload};
use crate::substrate::types::{
    AuthSetIdRpcResponse, AuthSetRpcResponse, BlockRpcResponse, HashRpcResponse, SignedBlock,
    SignedBlockWithAuthoritySet,
};
use crate::utils::{generate_client_id, to_string};
use bytes::buf::Buf;
use crossbeam_channel::{Receiver, Sender, TrySendError};
use futures::{future, FutureExt, SinkExt, StreamExt, TryFutureExt};
use hyper::{body::aggregate, Body, Client, Method, Request};
use log::*;
use parity_scale_codec::{Decode, Encode};
use parse_duration::parse;
use rand::Rng;
use serde_json::{from_str, Value};
use sp_core::sr25519::Pair as Sr25519Pair;
use sp_core::Pair;
use sp_finality_grandpa::AuthorityList;
use std::error::Error;
use std::marker::PhantomData;
use std::path::Path;
use substrate_subxt::balances::{Balances, BalancesEventsDecoder};
use substrate_subxt::system::{System, SystemEventsDecoder};
use substrate_subxt::{ClientBuilder, NodeTemplateRuntime, PairSigner};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[module]
pub trait TendermintClientModule: System + Balances {
    type Height: Encode + Decode + Default + Send + Sync + Copy + Clone + 'static;
}

#[derive(Clone, Debug, PartialEq, Call, Encode)]
pub struct InitClientCall<T: TendermintClientModule> {
    /// Runtime marker.
    pub _runtime: PhantomData<T>,
    /// Payload
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Call, Encode)]
pub struct UpdateClientCall<T: TendermintClientModule> {
    /// Runtime marker.
    pub _runtime: PhantomData<T>,
    /// Payload
    pub payload: Vec<u8>,
}

impl TendermintClientModule for NodeTemplateRuntime {
    type Height = u64;
}

pub struct SubstrateHandler {}

impl SubstrateHandler {
    pub async fn recv_handler(
        cfg: SubstrateChainConfig,
        outchan: Sender<SignedBlockWithAuthoritySet>,
    ) -> Result<(), String> {
        match cfg {
            SubstrateChainConfig::Real(cfg) => Self::chain_recv_handler(cfg, outchan).await,
            SubstrateChainConfig::Simulation(test_file) => {
                Self::simulate_recv_handler(test_file, outchan).await
            }
        }
    }

    pub async fn simulate_recv_handler(
        test_file: String,
        outchan: Sender<SignedBlockWithAuthoritySet>,
    ) -> Result<(), String> {
        let simulation_data =
            std::fs::read_to_string(Path::new(test_file.as_str())).map_err(to_string)?;
        let iterator = simulation_data.split("\n\n");
        for str in iterator {
            let payload: SignedBlockWithAuthoritySet = from_str(str).map_err(to_string)?;
            outchan.try_send(payload).map_err(to_string)?;
        }
        Ok(())
    }

    /// Subscribes to new blocks from Websocket, and pushes TMHeader objects into the Channel.
    pub async fn chain_recv_handler(
        cfg: SubstrateConfig,
        outchan: Sender<SignedBlockWithAuthoritySet>,
    ) -> Result<(), String> {
        let (mut socket, _) = connect_async(&cfg.ws_addr).await.map_err(to_string)?;
        info!("connected websocket to {:?}", &cfg.ws_addr);
        let subscribe_message = Message::Text(r#"{"jsonrpc":"2.0", "method":"chain_subscribeFinalizedHeads", "params":[], "id": "0"}"#.to_string());
        socket.send(subscribe_message).await.map_err(to_string)?;

        async fn process_msg(
            cfg: &SubstrateConfig,
            msg: Message,
        ) -> Result<SignedBlockWithAuthoritySet, String> {
            let msgtext = msg.to_text().map_err(to_string)?;
            let json = from_str::<Value>(msgtext).map_err(to_string)?;
            let blocknum = json["params"]["result"]["number"]
                .as_str()
                .map(|str| str.to_string())
                .ok_or_else(|| format!("ignoring json since it did not include the block number. Received json:{:?}", json))?;

            let (blockhash, block) = get_block_at_height(cfg.rpc_addr.clone(), blocknum.clone())
                .await
                .map_err(|e| {
                    format!("Unable to get block at height: {}, error: {}", blocknum, e)
                })?;

            let (authority_set, set_id) =
                get_authset_with_id(cfg.rpc_addr.clone(), blockhash.clone())
                    .await
                    .map_err(|e| {
                        format!(
                            "Unable to fetch authority set at height: {}, error: {}",
                            blocknum, e
                        )
                    })?;

            Ok(SignedBlockWithAuthoritySet::from_parts(
                block,
                authority_set,
                set_id,
            ))
        }

        while let Some(msg) = socket.next().await {
            if let Ok(msg) = msg {
                info!("Received message from substrate chain: {:?}", msg);
                match process_msg(&cfg, msg.clone()).await {
                    Ok(signed_block_with_authset) => outchan
                        .try_send(signed_block_with_authset)
                        .map_err(to_string)?,
                    Err(err) => error!("Error: {}", err),
                }
            }
        }

        Ok(())
    }

    fn generate_client_id(length: usize) -> String {
        let mut thread_rng = rand::rngs::ThreadRng::default();
        let mut id = String::new();
        for _ in 0..length {
            id.push(char::from(thread_rng.gen_range(97, 123)));
        }
        id
    }

    pub async fn send_handler(
        cfg: SubstrateChainConfig,
        client_id: Option<String>,
        inchan: Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
    ) -> Result<(), String> {
        match cfg {
            SubstrateChainConfig::Real(cfg) => {
                Self::chain_send_handler(cfg, client_id, inchan).await
            }
            SubstrateChainConfig::Simulation(_test_file) => Self::simulate_send_handler().await,
        }
    }

    // Nothing happens for now, as the simulation isn't interactive.
    pub async fn simulate_send_handler() -> Result<(), String> {
        Ok(())
    }

    pub async fn chain_send_handler(
        cfg: SubstrateConfig,
        client_id: Option<String>,
        inchan: Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
    ) -> Result<(), String> {
        let mut new_client = false;
        let id = if client_id.is_none() {
            new_client = true;
            generate_client_id()
        } else {
            client_id.unwrap()
        };
        let trusting_period = parse(cfg.trusting_period.as_str())
            .map_err(to_string)?
            .as_secs();
        let max_clock_drift = parse(cfg.max_clock_drift.as_str())
            .map_err(to_string)?
            .as_secs();
        let unbonding_period = parse(cfg.unbonding_period.as_str())
            .map_err(to_string)?
            .as_secs();
        let client_id = id.clone().parse().map_err(to_string)?;
        let (pair, _) = Sr25519Pair::from_phrase(cfg.signer_seed.as_str(), None)
            .map_err(|e| format!("{:?}", e))?;
        let signer = PairSigner::new(pair);
        let client = ClientBuilder::<NodeTemplateRuntime>::new()
            .set_url(cfg.ws_addr)
            .build()
            .await
            .map_err(to_string)?;
        loop {
            let result = inchan.try_recv();
            let msg = if result.is_err() {
                warn!("Did not receive any data from Cosmos receiver channel. Retrying in a second ...");
                tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                continue;
            } else {
                result.unwrap()
            };
            if new_client {
                new_client = false;
                let create_client_payload = TMCreateClientPayload {
                    header: msg.0,
                    trusting_period,
                    max_clock_drift,
                    unbonding_period,
                    client_id,
                };
                client
                    .init_client_and_watch(
                        &signer,
                        serde_json::to_vec(&create_client_payload).map_err(to_string)?,
                    )
                    .await
                    .map_err(to_string)?;
                info!("Created Cosmos light client");
            } else {
                let update_client_payload = TMUpdateClientPayload {
                    header: msg.0,
                    client_id: id.clone().parse().map_err(to_string)?,
                    next_validator_set: msg.1,
                };
                info!(
                    "{}",
                    format!(
                        "Updating Cosmos light client with block at height: {}",
                        update_client_payload.header.signed_header.header.height
                    )
                );
                client
                    .update_client_and_watch(
                        &signer,
                        serde_json::to_vec(&update_client_payload).map_err(to_string)?,
                    )
                    .await
                    .map_err(to_string)?;
                info!("Updated Cosmos light client");
            }
        }
    }
}

async fn get_block_at_height(
    rpc_addr: String,
    block_num: String,
) -> Result<(String, SignedBlock), Box<dyn Error>> {
    // TODO: just use the websocket for these requests.
    let client = Client::new();
    let block_hash_req = Request::builder()
        .method(Method::POST)
        .uri(rpc_addr.clone())
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0", "method": "chain_getBlockHash", "params": ["{}"], "id":"0"}}"#,
            block_num
        )))?;
    let response = client.request(block_hash_req).await?;
    let response_body = aggregate(response).await?;
    let stringified_body = String::from_utf8(response_body.bytes().to_vec())?;
    let block_hash_rpc_response: HashRpcResponse = from_str(&stringified_body)?;
    let block_request = Request::builder()
        .method(Method::POST)
        .uri(rpc_addr.clone())
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0", "method": "chain_getBlock", "params": ["{}"], "id":"0"}}"#,
            block_hash_rpc_response.result
        )))?;
    let response = client.request(block_request).await?;
    let response_body = aggregate(response).await?;
    let stringified_response = String::from_utf8(response_body.bytes().to_vec())?;
    let block_rpc_response: BlockRpcResponse = from_str(&stringified_response)?;
    info!("Got block at height: {}", block_num);
    Ok((block_hash_rpc_response.result, block_rpc_response.result))
}

async fn get_authset_with_id(
    rpc_addr: String,
    block_hash: String,
) -> Result<(AuthorityList, u64), Box<dyn Error>> {
    // TODO: just use the websocket for these requests.
    let client = Client::new();
    let set_req = Request::builder()
    .method(Method::POST)
    .uri(rpc_addr.clone())
    .header("content-type", "application/json")
    .body(Body::from(format!(r#"{{"jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x3a6772616e6470615f617574686f726974696573", "{}"], "id":"0"}}"#, block_hash)))?;

    let set_resp = client.request(set_req).await?;
    let set_body = aggregate(set_resp).await?;
    let set_rstr = String::from_utf8(set_body.bytes().to_vec())?;
    let set: AuthSetRpcResponse = from_str(&set_rstr).unwrap();

    let setid_req = Request::builder()
    .method(Method::POST)
    .uri(rpc_addr.clone())
    .header("content-type", "application/json")
    .body(Body::from(format!(r#"{{"jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x2371e21684d2fae99bcb4d579242f74a8a2d09463effcc78a22d75b9cb87dffc", "{}"], "id":"0"}}"#, block_hash)))?;
    let setid_resp = client.request(setid_req).await?;
    let setid_body = aggregate(setid_resp).await?;
    let setid_rstr = String::from_utf8(setid_body.bytes().to_vec())?;
    let setid_response: AuthSetIdRpcResponse = from_str(&setid_rstr)?;
    info!(
        "Received set id: {} and authority set: {:?} for block with hash: {}",
        setid_response.as_u64(),
        set.get_authset(),
        block_hash
    );
    Ok((set.get_authset(), setid_response.as_u64()))
}
