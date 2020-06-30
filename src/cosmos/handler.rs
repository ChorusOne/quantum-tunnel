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
use crate::utils::to_string;
use url::Url;
use signatory_secp256k1;
use signatory::ecdsa::SecretKey;
use crate::cosmos::crypto::{seed_from_mnemonic, privkey_from_seed};
use tendermint_light_client::PublicKey;
use signatory::public_key::PublicKeyed;
use std::borrow::Borrow;
use std::string::ToString;

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
    pub async fn recv_handler(cfg: CosmosConfig, outchan: Sender<TMHeader>) -> Result<(), String> {
        let rpc_url = Url::parse(&cfg.rpc_addr).map_err(to_string)?;
        let tm_addr = CosmosHandler::get_tm_addr(rpc_url);
        let mut client = Client::new(tm_addr.clone());
        info!("opening websocket to to {:?}", tm_addr.clone());
        let mut socket = EventListener::connect(tm_addr.clone()).await.map_err(to_string)?;

        info!("connected websocket to {:?}", tm_addr.clone());
        socket.subscribe(EventSubscription::BlockSubscription).await.map_err(to_string)?;
        loop {
            let response = Self::recv_data(&mut socket, &mut client, &outchan).await;
            if response.is_err() {
                error!("Error while processing tendermint node response: {}", response.err().unwrap());
            }
        }
    }

    async fn recv_data(socket: &mut EventListener, client: &mut Client, outchan: &Sender<TMHeader>) -> Result<(), Box<dyn Error>> {
        let maybe_result = socket.get_event().await?;
        if maybe_result.is_none() {
            // Return an error
        }
        let result = maybe_result.unwrap();
        match result.data {
            EventDataNewBlock(e) => {
                if e.block.is_none() {
                    // Return an error
                }
                let block = e.block.unwrap();
                let commit_future = client.commit(block.header.height);
                let validator_set_future = client.validators(block.header.height);
                let (signed_header_response, validator_set_response) = try_join!(commit_future, validator_set_future)?;
                let header = TMHeader {
                    signed_header: signed_header_response.signed_header,
                    validator_set: validator_set_response.validators,
                };
                outchan.try_send(header)?;
                info!(
                    "Processed incoming tendermint block for {:}",
                    block.header.height
                );
            }
            _ => {
                return Err("Unexpected type".into());
            }, // TODO: handle errors properly.
        }
        Ok(())
    }

    pub async fn send_handler(cfg: CosmosConfig, client_id: String, mut inchan: Receiver<SignedBlockWithAuthoritySet>) -> Result<(), String> {
        let key = privkey_from_seed(seed_from_mnemonic(cfg.seed.clone()).map_err(to_string)?);
        let signer =  signatory_secp256k1::EcdsaSigner::from(SecretKey::from_bytes(key).map_err(to_string)?.borrow());
        let pub_key = signer.public_key().map_err(to_string)?;
        let maybe_tm_pubkey =  PublicKey::from_raw_secp256k1(pub_key.as_bytes());
        if maybe_tm_pubkey.is_none() {
            return Err("Empty pubkey :/".into());
        }
        let tm_pubkey = maybe_tm_pubkey.unwrap();
        info!("{:?}", tm_pubkey.to_bech32("cosmos"));

        loop {
            let header = match inchan.try_recv() {
                Ok(val) => val,
                Err(e) => {
                    tokio::time::delay_for(core::time::Duration::new(1,0)).await;
                    continue;
                }
            };

            let msg = MsgUpdateWasmClient{
                header,
                client_id: client_id.clone(),
                address: tm_pubkey.to_bech32("cosmos"),
            };

            info!("{:?}", serde_json::to_string(&msg));
        }
    }

    pub async fn create_client(cfg: CosmosConfig, block: SignedBlockWithAuthoritySet) {}
}
