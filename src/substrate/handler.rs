use crate::config::SubstrateConfig;
use crate::cosmos::types::{TMCreateClientPayload, TMHeader, TMUpdateClientPayload};
use crate::substrate::types::{
    AuthSetIdRpcResponse, AuthSetRpcResponse, BlockRpcResponse, HashRpcResponse, SignedBlock,
    SignedBlockWithAuthoritySet,
};
use crate::utils::to_string;
use bytes::buf::Buf;
use crossbeam_channel::{Receiver, Sender};
use futures::{SinkExt, StreamExt};
use hyper::{body::aggregate, Body, Client, Method, Request};
use log::*;
use parity_scale_codec::{Decode, Encode};
use rand::Rng;
use serde_json::{from_str, Value};
use sp_finality_grandpa::AuthorityList;
use sp_keyring::AccountKeyring;
use std::error::Error;
use std::marker::PhantomData;
use substrate_subxt::balances::{Balances, BalancesEventsDecoder};
use substrate_subxt::system::{System, SystemEventsDecoder};
use substrate_subxt::DefaultNodeRuntime;
use substrate_subxt::{ClientBuilder, NodeTemplateRuntime, PairSigner};
use tendermint_light_client::{LightValidator, LightValidatorSet};
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
    /// Subscribes to new blocks from Websocket, and pushes TMHeader objects into the Channel.
    pub async fn recv_handler(
        cfg: SubstrateConfig,
        outchan: Sender<SignedBlockWithAuthoritySet>,
    ) -> Result<(), String> {
        let (mut socket, _) = connect_async(&cfg.ws_addr).await.map_err(to_string)?;
        info!("connected websocket to {:?}", &cfg.ws_addr);
        let subscribe_message = Message::Text(r#"{"jsonrpc":"2.0", "method":"chain_subscribeFinalizedHeads", "params":[], "id": "0"}"#.to_string());
        socket.send(subscribe_message).await.map_err(to_string)?;

        while let Some(msg) = socket.next().await {
            let msg = match msg {
                Err(e) => {
                    error!("Error on server stream: {:?}", e);

                    // Errors returned directly through the AsyncRead/Write API are fatal, generally an error on the underlying
                    // transport.
                    //
                    continue;
                }

                Ok(m) => m,
            };

            info!("server received: {}", msg);
            let msgtext = msg.to_text().ok().unwrap();
            let json_msg: Value = match from_str(msgtext) {
                Ok(val) => val,
                Err(e) => {
                    error!("Bad json unmarshal: {}", e);
                    continue;
                }
            };
            // let blocknum: BlockNumber = match json_msg["params"]["result"]["number"].as_str() {
            //     None => { error!("Didn't include a block number, ignoring..."); continue; },
            //     Some(x) => match u32::from_str_radix(&x[2..], 16) {
            //         Ok(val) => val,
            //         Err(e) => { error!("Unable to unmarshal blocknumber: {}", e); continue; }
            //     }
            //
            //
            // };
            let blocknum: String = match json_msg["params"]["result"]["number"].as_str() {
                None => {
                    error!("Didn't include a block number, ignoring...");
                    continue;
                }
                Some(x) => x.to_string(),
            };

            let (blockhash, block) =
                match get_block_at_height(cfg.rpc_addr.clone(), blocknum.clone()).await {
                    Ok(val) => val,
                    Err(e) => {
                        error!("Unable to get block at height: {}", e);
                        continue;
                    }
                };
            let (authset, set_id) =
                match get_authset_with_id(cfg.rpc_addr.clone(), blockhash.clone()).await {
                    Ok(val) => val,
                    Err(e) => {
                        error!("Unable to fetch authset: {}", e);
                        continue;
                    }
                };

            let sbwas = SignedBlockWithAuthoritySet::from_parts(block, authset, set_id);
            outchan.try_send(sbwas).map_err(to_string)?;
        }

        Ok(())

        // safe to drop the TCP connection
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
        cfg: SubstrateConfig,
        mut client_id: Option<String>,
        inchan: Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
    ) -> Result<(), String> {
        let mut new_client = false;
        let id = if client_id.is_none() {
            new_client = true;
            Self::generate_client_id(12)
        } else {
            client_id.unwrap()
        };
        let signer = PairSigner::new(AccountKeyring::Alice.pair());
        let client = ClientBuilder::<NodeTemplateRuntime>::new()
            .set_url(cfg.ws_addr)
            .build()
            .await
            .map_err(to_string)?;
        loop {
            let result = inchan.try_recv();
            let msg = if result.is_err() {
                tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                continue;
            } else {
                result.unwrap()
            };
            if new_client {
                new_client = true;
                let create_client_payload = TMCreateClientPayload {
                    header: msg.0,
                    trusting_period: cfg.trusting_period,
                    max_clock_drift: cfg.max_clock_drift,
                    unbonding_period: cfg.unbonding_period,
                    client_id: id.clone().parse().unwrap(),
                };
                client
                    .init_client_and_watch(
                        &signer,
                        serde_json::to_vec(&create_client_payload).unwrap(),
                    )
                    .await
                    .map_err(to_string)?;
            } else {
                let update_client_payload = TMUpdateClientPayload {
                    header: msg.0,
                    client_id: id.clone().parse().unwrap(),
                    next_validator_set: msg.1,
                };
                client
                    .update_client_and_watch(
                        &signer,
                        serde_json::to_vec(&update_client_payload).unwrap(),
                    )
                    .await
                    .map_err(to_string)?;
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
    let hash_req = Request::builder()
        .method(Method::POST)
        .uri(rpc_addr.clone())
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0", "method": "chain_getBlockHash", "params": ["{}"], "id":"0"}}"#,
            block_num
        )))?;

    //info!("req = {:?}", hash_req);
    // Await the response...
    let hash_resp = client.request(hash_req).await?;
    let hash_body = aggregate(hash_resp).await?;
    let hash_rstr = String::from_utf8(hash_body.bytes().to_vec()).unwrap(); // TODO: remove unwrap.
    let hash_response: HashRpcResponse = from_str(&hash_rstr).unwrap(); // TODO: remove unwrap.
                                                                        //info!("{}", hash_response.result);

    let block_req = Request::builder()
        .method(Method::POST)
        .uri(rpc_addr.clone())
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0", "method": "chain_getBlock", "params": ["{}"], "id":"0"}}"#,
            hash_response.result
        )))?;
    //info!("req(2) = {:?}", block_req);
    let block_resp = client.request(block_req).await?;
    let block_body = aggregate(block_resp).await?;
    let block_rstr = String::from_utf8(block_body.bytes().to_vec()).unwrap(); // TODO: remove unwrap.
    let block_response: BlockRpcResponse = from_str(&block_rstr).unwrap(); // TODO: remove unwrap.
                                                                           //info!("{:#?}", block_response.result);

    info!("Got block for {}", block_num);
    Ok((hash_response.result, block_response.result))
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

    //info!("req = {:?}", set_req);
    // Await the response...
    let set_resp = client.request(set_req).await?;
    let set_body = aggregate(set_resp).await?;
    let set_rstr = String::from_utf8(set_body.bytes().to_vec()).unwrap(); // TODO: remove unwrap.
    let set: AuthSetRpcResponse = from_str(&set_rstr).unwrap();

    let setid_req = Request::builder()
    .method(Method::POST)
    .uri(rpc_addr.clone())
    .header("content-type", "application/json")
    .body(Body::from(format!(r#"{{"jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x2371e21684d2fae99bcb4d579242f74a8a2d09463effcc78a22d75b9cb87dffc", "{}"], "id":"0"}}"#, block_hash)))?;
    //info!("req(2) = {:?}", setid_req);
    let setid_resp = client.request(setid_req).await?;
    let setid_body = aggregate(setid_resp).await?;
    let setid_rstr = String::from_utf8(setid_body.bytes().to_vec()).unwrap(); // TODO: remove unwrap.
    info!("{}", setid_rstr);

    let setid_response: AuthSetIdRpcResponse = from_str(&setid_rstr).unwrap(); // TODO: remove unwrap.
                                                                               //info!("{:#?}", setid_response.result);
    info!("Got authset for {}", block_hash);
    Ok((set.get_authset(), setid_response.as_u64()))
}
