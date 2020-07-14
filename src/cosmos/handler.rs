use crate::config::CosmosConfig;
use crate::cosmos::crypto::{privkey_from_seed, seed_from_mnemonic};
use crate::cosmos::types::{
    AccountQueryResponse, DecCoin, MsgCreateWasmClient, MsgUpdateWasmClient, StdFee, StdMsg,
    StdSignature, StdTx, TMHeader, TxRpcResponse,
};
use crate::error::ErrorKind::{MalformedResponse, UnexpectedPayload};
use crate::substrate::types::SignedBlockWithAuthoritySet;
use crate::utils::{create_client_id, to_string};
use bytes::buf::Buf;
use crossbeam_channel::{Receiver, Sender};
use futures::try_join;
use hyper::{body::aggregate, Body, Client as HClient, Method, Request};
use log::*;
use parse_duration::parse;
use signatory::ecdsa::SecretKey;
use signatory::public_key::PublicKeyed;
use signatory_secp256k1;
use std::error::Error;
use std::string::ToString;
use subtle_encoding::bech32;
use tendermint::net::Address;
use tendermint_light_client::{AccountId, LightValidator, LightValidatorSet, PublicKey};
use tendermint_rpc::{
    event_listener::{EventListener, EventSubscription, TMEventData::EventDataNewBlock},
    Client,
};
use url::Url;

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
    pub async fn recv_handler(
        cfg: CosmosConfig,
        outchan: Sender<(TMHeader, LightValidatorSet<LightValidator>)>,
    ) -> Result<(), String> {
        let rpc_url = Url::parse(&cfg.rpc_addr).map_err(to_string)?;
        let tm_addr = CosmosHandler::get_tm_addr(rpc_url);
        let mut client = Client::new(tm_addr.clone());
        info!("opening websocket to to {:?}", tm_addr.clone());
        let mut socket = EventListener::connect(tm_addr.clone())
            .await
            .map_err(to_string)?;

        info!("connected websocket to {:?}", tm_addr.clone());
        socket
            .subscribe(EventSubscription::BlockSubscription)
            .await
            .map_err(to_string)?;
        let mut previous_block: Option<TMHeader> = None;
        loop {
            let response = Self::recv_data(&mut socket, &mut client).await;
            if response.is_err() {
                error!("Error while processing tendermint node response.");
            }
            let header = response.map_err(to_string)?;
            if previous_block.is_none() {
                previous_block = Some(header);
                continue;
            }
            outchan
                .try_send((previous_block.unwrap(), header.validator_set.clone()))
                .map_err(to_string)?;
            previous_block = Some(header);
        }
    }

    fn convert_to_light_validator_set(
        validators: Vec<tendermint::validator::Info>,
    ) -> LightValidatorSet<LightValidator> {
        let mut light_validators: Vec<LightValidator> = vec![];
        for validator in validators {
            let light_pub_key = match validator.pub_key {
                tendermint::public_key::PublicKey::Ed25519(key) => {
                    tendermint_light_client::PublicKey::Ed25519(key)
                }
                tendermint::public_key::PublicKey::Secp256k1(key) => {
                    tendermint_light_client::PublicKey::Secp256k1(key)
                }
            };
            let light_vote_power =
                tendermint_light_client::VotePower::new(validator.voting_power.value());
            light_validators.push(LightValidator::new(light_pub_key, light_vote_power));
        }
        LightValidatorSet::new(light_validators)
    }

    async fn recv_data(
        socket: &mut EventListener,
        client: &mut Client,
    ) -> Result<TMHeader, Box<dyn Error>> {
        let maybe_result = socket.get_event().await?;
        if maybe_result.is_none() {
            // Return an error
        }
        let result = maybe_result.unwrap();
        match result.data {
            EventDataNewBlock(e) => {
                if e.block.is_none() {
                    return Err(MalformedResponse("e.block".into()).into());
                }
                let block = e.block.unwrap();
                let commit_future = client.commit(block.header.height);
                let validator_set_future = client.validators(block.header.height);
                let (signed_header_response, validator_set_response) =
                    try_join!(commit_future, validator_set_future)?;
                let header = TMHeader {
                    signed_header: signed_header_response.signed_header,
                    validator_set: Self::convert_to_light_validator_set(
                        validator_set_response.validators,
                    ),
                };
                info!(
                    "Processed incoming tendermint block for {:}",
                    block.header.height
                );
                Ok(header)
            }
            _ => Err(UnexpectedPayload.into()),
        }
    }

    fn signer_from_seed(
        seed: String,
    ) -> Result<(signatory_secp256k1::EcdsaSigner, PublicKey, String), String> {
        let key = seed_from_mnemonic(seed).map_err(to_string)?;
        let secret_key = SecretKey::from_bytes(privkey_from_seed(key)).map_err(to_string)?;
        let signer = signatory_secp256k1::EcdsaSigner::from(&secret_key);
        let tmpubkey = PublicKey::from(signer.public_key().map_err(to_string)?);
        let address = bech32::encode("cosmos", AccountId::from(tmpubkey).as_bytes());
        Ok((signer, tmpubkey, address))
    }

    pub async fn send_handler(
        cfg: CosmosConfig,
        client_id: Option<String>,
        inchan: Receiver<SignedBlockWithAuthoritySet>,
    ) -> Result<(), String> {
        let client_id = match client_id {
            Some(val) => val,
            None => {
                // if we don't pass in an existing client_id, then try to fetch the first header, and send a create client message.
                loop {
                    match inchan.try_recv() {
                        Ok(val) => break CosmosHandler::create_client(cfg.clone(), val).await?,
                        Err(_) => {
                            tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                            continue;
                        }
                    }
                }
            }
        };

        loop {
            let header = match inchan.try_recv() {
                Ok(val) => val,
                Err(_) => {
                    tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                    continue;
                }
            };

            let txhash =
                CosmosHandler::update_client(cfg.clone(), header, client_id.clone()).await?;

            info!("Update TxHash: {:?}", txhash);

            if false {
                break;
            }
        }

        Ok(())
    }

    pub async fn create_client(
        cfg: CosmosConfig,
        header: SignedBlockWithAuthoritySet,
    ) -> Result<String, String> {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.seed.clone()).map_err(to_string)?;

        let client_id = create_client_id();

        let msg = MsgCreateWasmClient {
            header: header,
            address: address.clone(),
            trusting_period: parse(&cfg.trusting_period).unwrap().as_nanos().to_string(),
            max_clock_drift: parse(&cfg.max_clock_drift).unwrap().as_nanos().to_string(),
            unbonding_period: parse(&cfg.unbonding_period).unwrap().as_nanos().to_string(),
            client_id: client_id.clone(),
            wasm_id: cfg.wasm_id,
        };

        let m = vec![serde_json::json!({"type": MsgCreateWasmClient::get_type(), "value": &msg})];
        let f = StdFee {
            gas: cfg.gas,
            amount: vec![DecCoin::from(cfg.gas_price).mul(cfg.gas as f64).to_coin()],
        };

        let retval = CosmosHandler::submit_tx(
            m,
            f,
            "".to_owned(),
            signer,
            address.clone(),
            cfg.chain_id.clone(),
            cfg.lcd_addr.clone(),
        )
        .await
        .map_err(to_string)?;
        info!("Create TxHash: {:?}", retval);
        Ok(client_id.clone())
    }

    pub async fn update_client(
        cfg: CosmosConfig,
        header: SignedBlockWithAuthoritySet,
        client_id: String,
    ) -> Result<String, String> {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.seed.clone()).map_err(to_string)?;

        let msg = MsgUpdateWasmClient {
            header: header,
            address: address.clone(),
            client_id: client_id.clone(),
        };

        let m = vec![serde_json::json!({"type": MsgUpdateWasmClient::get_type(), "value": &msg})];
        let f = StdFee {
            gas: cfg.gas,
            amount: vec![DecCoin::from(cfg.gas_price).mul(cfg.gas as f64).to_coin()],
        };

        let retval = CosmosHandler::submit_tx(
            m,
            f,
            "".to_owned(),
            signer,
            address.clone(),
            cfg.chain_id.clone(),
            cfg.lcd_addr.clone(),
        )
        .await
        .map_err(to_string)?;

        Ok(retval)
    }

    async fn submit_tx(
        msgs: Vec<serde_json::Value>,
        fee: StdFee,
        memo: String,
        signer: signatory_secp256k1::EcdsaSigner,
        address: String,
        chain_id: String,
        lcd_addr: String,
    ) -> Result<String, String> {
        let mut tx = StdTx {
            msg: msgs.to_vec(),
            fee: fee,
            signatures: vec![],
            memo: memo,
        };

        let (accnum, sequence) = CosmosHandler::get_account(address, lcd_addr.clone()).await?;
        let bytes_to_sign = tx.get_sign_bytes(chain_id, accnum, sequence);
        let sig_block = StdSignature::sign(signer, bytes_to_sign);
        tx.signatures.push(sig_block.clone());
        let wrapped_tx = serde_json::json!({"tx": &tx, "mode":"block", "account_number": &accnum.to_string(), "sequence": &sequence.to_string()});

        let json_bytes = serde_json::to_vec(&wrapped_tx).map_err(to_string)?;

        let hclient = HClient::new();
        let tx_req = Request::builder()
            .method(Method::POST)
            .uri(lcd_addr.clone() + &"txs".to_owned())
            .header("content-type", "application/json")
            .body(Body::from(json_bytes))
            .map_err(to_string)?;

        // Await the response...
        let tx_resp = hclient.request(tx_req).await.map_err(to_string)?;
        let tx_body = aggregate(tx_resp).await.map_err(to_string)?;
        let tx_rstr = String::from_utf8(tx_body.bytes().to_vec()).map_err(to_string)?;
        let tx_response: TxRpcResponse = serde_json::from_str(&tx_rstr).map_err(to_string)?;
        if tx_response.code != 0 {
            error!("Tx failed: {:?}", tx_response.raw_log);
            return Err(tx_response.txhash);
        };
        Ok(tx_response.txhash)
    }

    async fn get_account(account: String, lcd_addr: String) -> Result<(u64, u64), String> {
        let hclient = HClient::new();
        let acc_req = Request::builder()
            .method(Method::GET)
            .uri(lcd_addr.clone() + &"auth/accounts/".to_owned() + &account)
            .header("content-type", "application/json")
            .body(Body::from(""))
            .map_err(to_string)?;

        // Await the response...
        let acc_resp = hclient.request(acc_req).await.map_err(to_string)?;
        let acc_body = aggregate(acc_resp).await.map_err(to_string)?;
        let acc_rstr = String::from_utf8(acc_body.bytes().to_vec()).map_err(to_string)?;
        let response: AccountQueryResponse = serde_json::from_str(&acc_rstr).unwrap();
        Ok((
            response.result.value.account_number,
            response.result.value.sequence,
        ))
    }
}
