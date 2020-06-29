use crate::config::CosmosConfig;
use crate::cosmos::types::{TMHeader, MsgUpdateWasmClient, MsgCreateWasmClient};
use crate::substrate::types::SignedBlockWithAuthoritySet;
use crossbeam_channel::{Receiver, Sender};
use futures::{
    try_join,
};
use simple_error::SimpleError;
use std::error::Error;
use log::*;
use tendermint::net::Address;
use tendermint_rpc::{
    event_listener::{EventListener, EventSubscription, TMEventData::EventDataNewBlock},
    Client,
};
use serde_json::to_string;
use url::Url;
use signatory_secp256k1;
use signatory::ecdsa::SecretKey;
use crate::cosmos::crypto::{seed_from_mnemonic, privkey_from_seed};
use tendermint_light_client::PublicKey;
use signatory::public_key::PublicKeyed;

pub struct CosmosHandler {}
impl CosmosHandler {

    fn get_tm_addr(url: Url) -> Address {
        Address::Tcp {
            host: url.host_str().unwrap().to_string(),
            port: url.port().unwrap(),
            peer_id: None,
        }
    }

    /// Subscribes to new blocks from Websocket, and pushes TMHeader objects into the Channel.
    pub async fn recv_handler(cfg: CosmosConfig, outchan: Sender<TMHeader>) {
        let rpc_url = match Url::parse(&cfg.rpc_addr) {
            Ok(val) => val,
            Err(e) => {
                error!("{}", e.to_string());
                return;
            }
        };

        let tm_addr = CosmosHandler::get_tm_addr(rpc_url);

        let client = Client::new(tm_addr.clone());
        info!("opening websocket to to {:?}", tm_addr.clone());
        let mut socket = match EventListener::connect(tm_addr.clone())
            .await {
                Ok(val) => { info!("raa"); val },
                Err(e) => {
                    error!("{}", e.to_string());
                    return;
                }
            };

        info!("connected websocket to {:?}", tm_addr.clone());
        socket
            .subscribe(EventSubscription::BlockSubscription)
            .await;
        loop {
            let result = socket.get_event().await;
            match result {
                Err(e) => {
                    warn!("received something unexpected");
                    continue;
                } // TODO: handle errors properly.
                Ok(res) => match res {
                    None => error!("No block"),
                    Some(block) => {
                        match block.data {
                            EventDataNewBlock(e) => {
                                match e.block {
                                    Some(block) => {
                                        // TODO: better naming of some of these return values!
                                        let commit_fut = client.commit(block.header.height);
                                        let vs_fut = client.validators(block.header.height);
                                        let r2 = try_join!(commit_fut, vs_fut);
                                        match r2 {
                                            Err(e) => {
                                                error!("Unable to fetch packet parts from rpc");
                                                return;
                                            }
                                            Ok(r3) => {
                                                let h = TMHeader {
                                                    signed_header: r3.0.signed_header,
                                                    validator_set: r3.1.validators,
                                                };
                                                outchan.try_send(h);
                                                info!(
                                                    "Processed incoming tendermint block for {:}",
                                                    block.header.height
                                                );
                                            }
                                        }
                                    }
                                    None => {
                                        error!("No block (2)");
                                        continue;
                                    }, // TODO: handle errors properly.
                                }
                            }
                            _ => {
                                error!("Unexpected type");
                                continue;
                            }, // TODO: handle errors properly.
                        }
                    }
                },
            }
        }
    }

    pub async fn send_handler(cfg: CosmosConfig, client_id: String, mut inchan: Receiver<SignedBlockWithAuthoritySet>) {
        let key = match seed_from_mnemonic(cfg.seed.clone()) {
            Ok(val) => privkey_from_seed(val),
            Err(e) => {
                error!("Unable to create key from seed");
                return;
            }
        };

        let signer = match SecretKey::from_bytes(key) {
            Ok(val) => signatory_secp256k1::EcdsaSigner::from(&val),
            Err(e) => {
                error!("Unable to create signer");
                return;
            }
        };
        let pubkey = match signer.public_key() {
            Ok(val) => val,
            Err(e) => {
                error!("Unable to determine pubkey");
                return;
            }
        };

        let tmpubkey = match PublicKey::from_raw_secp256k1(pubkey.as_bytes()) {
            Some(val) => val,
            None => {
                error!("Empty pubkey :/");
                return;
            }
        };
        info!("{:?}", tmpubkey.to_bech32("cosmos"));

        loop {
            let header = match inchan.try_recv() {
                Ok(val) => val,
                Err(e) => {
                    tokio::time::delay_for(core::time::Duration::new(1,0)).await;
                    continue;
                }
            };

            let msg = MsgUpdateWasmClient{
                header: header,
                client_id: client_id.clone(),
                address: tmpubkey.to_bech32("cosmos"),
            };

            info!("{:?}", serde_json::to_string(&msg));
        }
    }

    pub async fn create_client(cfg: CosmosConfig, block: SignedBlockWithAuthoritySet) {}
}
