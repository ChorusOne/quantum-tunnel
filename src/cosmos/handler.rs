use crate::config::CosmosConfig;
use crate::cosmos::types::{TMHeader, MsgUpdateWasmClient, MsgCreateWasmClient, StdTx, StdFee, DecCoin};
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
use crate::utils::{to_string, create_client_id};
use url::Url;
use signatory_secp256k1;
use signatory::ecdsa::SecretKey;
use crate::cosmos::crypto::{seed_from_mnemonic, privkey_from_seed};
use tendermint_light_client::{PublicKey, Id};
use signatory::public_key::PublicKeyed;
use std::borrow::Borrow;
use std::string::ToString;
use std::str::from_utf8;
use subtle_encoding::bech32;
use parse_duration::parse;

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

    fn signer_from_seed(seed: String) -> Result<(signatory_secp256k1::EcdsaSigner, String), String> {
        let key = seed_from_mnemonic(seed).map_err(to_string)?;
        let secret_key = SecretKey::from_bytes(privkey_from_seed(key)).map_err(to_string)?;
        let signer = signatory_secp256k1::EcdsaSigner::from(&secret_key);
        let tmpubkey = PublicKey::from(signer.public_key().map_err(to_string)?);
        let address = bech32::encode("cosmos", Id::from(tmpubkey).as_bytes());
        info!("Sender address: {:?}", address.clone());
        Ok((signer, address))
    }

    pub async fn send_handler(cfg: CosmosConfig, client_id: Option<String>, mut inchan: Receiver<SignedBlockWithAuthoritySet>) -> Result<(), String> {

        let (signer, address) = CosmosHandler::signer_from_seed(cfg.seed.clone()).map_err(to_string)?;
        let client_id = match client_id {
            Some(val) => val,
            None => {
                // if we don't pass in an existing client_id, then try to fetch the first header, and send a create client message.
                loop {
                    match inchan.try_recv() {
                        Ok(val) => {
                            break CosmosHandler::create_client(cfg.clone(), val).await?
                        },
                        Err(e) => {
                            tokio::time::delay_for(core::time::Duration::new(1,0)).await;
                            continue;
                        }
                    }
                }
            }
        };

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
                address: address.clone(),
            };

            info!("{:?}", serde_json::to_string(&msg));

            if (false) {
                break;
            }
        };

        Ok(())
    }


    pub async fn create_client(cfg: CosmosConfig, header: SignedBlockWithAuthoritySet) -> Result<String, String> {
        let (signer, address) = CosmosHandler::signer_from_seed(cfg.seed.clone()).map_err(to_string)?;

        let client_id = create_client_id();

        let msg = MsgCreateWasmClient{
            header: header,
            address: address,
            trusting_period: parse(&cfg.trusting_period).unwrap().as_nanos().to_string(),
            max_clock_drift: parse(&cfg.max_clock_drift).unwrap().as_nanos().to_string(),
            unbonding_period: parse(&cfg.unbonding_period).unwrap().as_nanos().to_string(),
            client_id: client_id.clone(),
            wasm_id: cfg.wasm_id,
        };

        let m = vec![serde_json::json!(msg)];
        let f = StdFee{
            gas: cfg.gas,
            amount: vec![DecCoin::from(cfg.gas_price).mul(cfg.gas as f64).to_coin()],
        };

        let tx = StdTx{
            msg: m,
            fee: f,
            signatures: vec![],
            memo: "Oh hai".to_owned(),
        };

        info!("{:?}", from_utf8(tx.get_sign_bytes("test".to_string(), 0, 0).as_slice()).unwrap());

        //info!("{:?}", serde_json::to_string(&msg));

        Ok(client_id.clone())
    }
}
