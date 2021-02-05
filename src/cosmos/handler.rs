use crate::config::{CosmosChainConfig, CosmosConfig};
use crate::cosmos::crypto::{privkey_from_seed, seed_from_mnemonic};
use crate::cosmos::types::simulation::Message;
use crate::cosmos::types::{
    AccountQueryResponse, DecCoin, MsgCreateWasmClient, MsgUpdateWasmClient, StdFee, StdMsg,
    StdSignature, StdTx, TMHeader, TxRpcResponse,
};
use crate::error::ErrorKind;
use crate::error::ErrorKind::{MalformedResponse, UnexpectedPayload};
use crate::substrate::types::{CreateSignedBlockWithAuthoritySet, SignedBlockWithAuthoritySet};
use crate::utils::{generate_client_id, to_string};
use bytes::buf::Buf;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use futures::try_join;
use hyper::{body::aggregate, Body, Client as HClient, Method, Request};
use log::*;
use parse_duration::parse;
use k256::{elliptic_curve::SecretKey, ecdsa::SigningKey};
use k256::EncodedPoint as Secp256k1;
use std::error::Error;
use std::path::Path;
use std::string::ToString;
use subtle_encoding::bech32;
use tendermint::net::Address;
use tendermint_light_client::{AccountId, PublicKey};
use tendermint_rpc::{WebSocketClient, SubscriptionClient, Client};
use tendermint_rpc::query::EventType;
use futures::StreamExt;
use url::Url;

pub struct CosmosHandler {}
impl CosmosHandler {
    fn parse_tm_addr(url: Url) -> Result<Address, String> {
        if url.host_str().is_none() {
            return Err(format!("missing host string in url: {}", url));
        }
        if url.port().is_none() {
            return Err(format!("missing port in url: {}", url));
        }
        Ok(Address::Tcp {
            host: url.host_str().unwrap().to_string(),
            port: url.port().unwrap(),
            peer_id: None,
        })
    }

    /// Receive handler entrypoint
    /// Branches to different internal methods depending upon whether
    /// configuration is `Real` or `Simulation`
    pub async fn recv_handler(
        cfg: CosmosChainConfig,
        outchan: Sender<(TMHeader, Vec<tendermint::validator::Info>)>,
        monitoring_inchan: Receiver<(bool, u64)>,
    ) -> Result<(), String> {
        match cfg {
            CosmosChainConfig::Real(cfg) => Self::chain_recv_handler(cfg, outchan).await,
            CosmosChainConfig::Simulation(cfg) => {
                Self::simulate_recv_handler(
                    cfg.simulation_file_path,
                    cfg.should_run_till_height,
                    outchan,
                    monitoring_inchan,
                )
                .await
            }
        }
    }

    /// Simulation receive handler, which as the name suggests
    /// take the chain headers from simulation target instead of
    /// live chain. It also monitors send handler of opposite chain to detect
    /// whether or not simulation is successful.
    pub async fn simulate_recv_handler(
        test_file: String,
        should_run_till_height: u64,
        outchan: Sender<(TMHeader, Vec<tendermint::validator::Info>)>,
        monitoring_inchan: Receiver<(bool, u64)>,
    ) -> Result<(), String> {
        let simulation_data =
            std::fs::read_to_string(Path::new(test_file.as_str())).map_err(to_string)?;
        let stringified_headers: Vec<&str> = simulation_data.split("\n\n").collect();
        let number_of_simulated_headers = stringified_headers.len();
        for str in stringified_headers {
            let payload: Message = serde_json::from_str(str).map_err(to_string)?;
            outchan
                .try_send((payload.header, payload.next_validators))
                .map_err(to_string)?;
        }

        let mut number_of_headers_ingested_till = 0;
        let mut successfully_ingested_till = 0;
        // Let's wait for the receive handler on other side to catch up
        loop {
            let result = monitoring_inchan.try_recv();
            if result.is_err() {
                match result.err().unwrap() {
                    TryRecvError::Empty => {
                        // Let's wait for data to appear
                        tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                    }
                    TryRecvError::Disconnected => {
                        return Err(
                            "monitoring channel of substrate send handler is disconnected"
                                .to_string(),
                        );
                    }
                }
                continue;
            }

            let (terminated, reported_height) = result.unwrap();
            if !terminated {
                successfully_ingested_till = reported_height;
                number_of_headers_ingested_till += 1;
            }

            if terminated || (number_of_headers_ingested_till == number_of_simulated_headers) {
                if successfully_ingested_till != should_run_till_height {
                    return Err(format!("Ingesting simulation data failed on cosmos chain. Expected to ingest headers till height: {}, ingested till: {}", should_run_till_height, successfully_ingested_till));
                } else {
                    info!(
                        "Cosmos headers simulated successfully. Ingested headers till height: {}",
                        successfully_ingested_till
                    );
                }
                break;
            } else {
                info!(
                    "cosmos light client has successfully ingested header at: {}",
                    successfully_ingested_till
                );
            }
        }
        Ok(())
    }

    /// Subscribes to new blocks from Websocket, and pushes TMHeader objects into the Channel.
    pub async fn chain_recv_handler(
        cfg: CosmosConfig,
        outchan: Sender<(TMHeader, Vec<tendermint::validator::Info>)>,
    ) -> Result<(), String> {
        let rpc_url = Url::parse(&cfg.rpc_addr).map_err(to_string)?;
        let tm_addr = CosmosHandler::parse_tm_addr(rpc_url)?;
        info!("opening websocket to to {:?}", tm_addr.clone());
        let (mut client, driver) = WebSocketClient::new(tm_addr.clone())
            .await
            .map_err(to_string)?;
        let driver_handle = tokio::spawn(async move { driver.run().await });

        info!("connected websocket to {:?}", tm_addr.clone());
        let mut subs = client
            .subscribe(EventType::NewBlock.into())
            .await
            .map_err(to_string)?;
        let mut previous_block: Option<TMHeader> = None;

        while let Some(response) = subs.next().await {
            let response = Self::recv_data(response, &mut client).await;
            if response.is_err() {
                error!(
                    "Error: {} while processing tendermint node response",
                    response.err().unwrap()
                );
                continue;
            }
            let header = response.unwrap();
            if previous_block.is_none() {
                previous_block = Some(header);
                continue;
            }
            outchan
                .try_send((previous_block.unwrap(), header.validator_set.clone()))
                .map_err(to_string)?;
            previous_block = Some(header);
        }

        // Signal to the driver to terminate.
        let _ = client.close().map_err(to_string);

        // Await the driver's termination to ensure proper connection closure.
        driver_handle.await.unwrap().map_err(to_string)
    }

    async fn recv_data(
        response: Result<tendermint_rpc::event::Event, tendermint_rpc::Error>,
        client: &mut WebSocketClient,
    ) -> Result<TMHeader, Box<dyn Error>> {
        let maybe_result = response;
        if maybe_result.is_err() {
            return Err(ErrorKind::Io("unable to get events from socket".to_string()).into());
        }
        let result = maybe_result.unwrap();
        match result.data {
            tendermint_rpc::event::EventData::NewBlock {
                    block,
                    result_begin_block: _,
                    result_end_block: _,
            } => {
                if block.is_none() {
                    return Err(MalformedResponse("e.block".into()).into());
                }
                let block = block.unwrap();
                let commit_future = client.commit(block.header.height);
                let validator_set_future = client.validators(block.header.height);
                let (signed_header_response, validator_set_response) =
                    try_join!(commit_future, validator_set_future)?;
                let header = TMHeader {
                    signed_header: signed_header_response.signed_header,
                    validator_set: validator_set_response.validators,
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
    ) -> Result<(SigningKey, PublicKey, String), String> {
        let key = seed_from_mnemonic(seed).map_err(to_string)?;
        let secret_key = SecretKey::from_bytes(privkey_from_seed(key)).map_err(to_string)?;
        let signing_key = SigningKey::from(&secret_key);
        let tmpubkey = PublicKey::from(Secp256k1::from_secret_key(&secret_key, true));
        let address = bech32::encode("cosmos", AccountId::from(tmpubkey).as_bytes());
        Ok((signing_key, tmpubkey, address))
    }

    /// Send handler entrypoint
    /// Branches to different internal methods depending upon whether
    /// configuration is `Real` or `Simulation`
    /// If other side is simulation, some additional bookkeeping is done to
    /// make sure `simulation_recv_handler` gets accurate data.
    pub async fn send_handler(
        cfg: CosmosChainConfig,
        client_id: Option<String>,
        inchan: Receiver<SignedBlockWithAuthoritySet>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> {
        match cfg {
            CosmosChainConfig::Real(cfg) => {
                if cfg.is_other_side_simulation {
                    // Swallow up the error to prevent quantum tunnel to terminate. This will give simulation data reader the chance to print the result.
                    let result = Self::chain_send_handler(
                        cfg,
                        client_id,
                        inchan,
                        monitoring_outchan.clone(),
                    )
                    .await;
                    // Send signal to simulation_recv_handler that receive handler is terminated
                    monitoring_outchan.try_send((true, 0)).map_err(to_string)?;
                    if result.is_err() {
                        error!("Error occurred while trying to send simulated cosmos data to cosmos chain: {}", result.err().unwrap());
                    }
                    // This gives simulation_recv_handler time to print result and then exit.
                    futures::future::pending::<()>().await;
                    Ok(())
                } else {
                    Self::chain_send_handler(cfg, client_id, inchan, monitoring_outchan).await
                }
            }
            // If we are running simulation, we just drain incoming headers.
            CosmosChainConfig::Simulation(_cfg) => {
                loop {
                    let result = inchan.try_recv();
                    if result.is_err() {
                        match result.err().unwrap() {
                            TryRecvError::Disconnected => {
                                return Err(
                                    "Substrate chain-data channel's input end is disconnected."
                                        .to_string(),
                                );
                            }
                            _ => {}
                        }
                    }
                    // Compulsory delay of 1 second to prevent busy loop.
                    tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                }
            }
        }
    }

    /// Transforms header data received from opposite chain to
    /// light client payload and sends it to substrate light client running in
    /// cosmos chain.
    /// If client id is not passed, first payload sent would be for creating the client.
    pub async fn chain_send_handler(
        cfg: CosmosConfig,
        client_id: Option<String>,
        inchan: Receiver<SignedBlockWithAuthoritySet>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> {
        let mut new_client = false;
        let id = if client_id.is_none() {
            new_client = true;
            generate_client_id()
        } else {
            client_id.unwrap()
        };

        loop {
            let result = inchan.try_recv();
            let msg = if result.is_err() {
                match result.err().unwrap() {
                    TryRecvError::Disconnected => {
                        return Err(
                            "Substrate chain-data channel's input end is disconnected.".to_string()
                        );
                    }
                    _ => {
                        warn!("Did not receive any data from Substrate chain-data channel. Retrying in a second ...");
                        tokio::time::delay_for(core::time::Duration::new(1, 0)).await;
                        continue;
                    }
                }
            } else {
                result.unwrap()
            };

            let current_height = msg.block.block.header.number;

            if new_client {
                new_client = false;
                CosmosHandler::create_client(cfg.clone(), id.clone(), msg).await?;
            } else {
                CosmosHandler::update_client(cfg.clone(), msg, id.clone()).await?;
            }

            if cfg.is_other_side_simulation {
                monitoring_outchan
                    .try_send((false, current_height as u64))
                    .map_err(to_string)?;
            }
        }
    }

    pub async fn create_client(
        cfg: CosmosConfig,
        client_id: String,
        header: SignedBlockWithAuthoritySet,
    ) -> Result<String, String> {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.signer_seed.clone()).map_err(to_string)?;

        let msg = MsgCreateWasmClient {
            header: CreateSignedBlockWithAuthoritySet {
                block: header.block,
                authority_set: header.authority_set,
                set_id: header.set_id,
                max_headers_allowed_to_store: 256,
                max_headers_allowed_between_justifications: 512,
            },
            address: address.clone(),
            trusting_period: parse(&cfg.trusting_period)
                .map_err(to_string)?
                .as_nanos()
                .to_string(),
            max_clock_drift: parse(&cfg.max_clock_drift)
                .map_err(to_string)?
                .as_nanos()
                .to_string(),
            unbonding_period: parse(&cfg.unbonding_period)
                .map_err(to_string)?
                .as_nanos()
                .to_string(),
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
        info!("Substrate light client creation TxHash: {:?}", retval);
        Ok(client_id.clone())
    }

    pub async fn update_client(
        cfg: CosmosConfig,
        header: SignedBlockWithAuthoritySet,
        client_id: String,
    ) -> Result<String, String> {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.signer_seed.clone()).map_err(to_string)?;

        let msg = MsgUpdateWasmClient {
            header,
            address: address.clone(),
            client_id: client_id.clone(),
        };

        let msgs =
            vec![serde_json::json!({"type": MsgUpdateWasmClient::get_type(), "value": &msg})];
        let txfee = StdFee {
            gas: cfg.gas,
            amount: vec![DecCoin::from(cfg.gas_price).mul(cfg.gas as f64).to_coin()],
        };

        let retval = CosmosHandler::submit_tx(
            msgs,
            txfee,
            "".to_owned(),
            signer,
            address.clone(),
            cfg.chain_id.clone(),
            cfg.lcd_addr.clone(),
        )
        .await
        .map_err(to_string)?;
        info!("Substrate light client updation TxHash: {:?}", retval);
        Ok(retval)
    }

    async fn submit_tx(
        msgs: Vec<serde_json::Value>,
        fee: StdFee,
        memo: String,
        signer: SigningKey,
        address: String,
        chain_id: String,
        lcd_addr: String,
    ) -> Result<String, String> {
        let mut tx = StdTx {
            msg: msgs.to_vec(),
            fee,
            signatures: vec![],
            memo,
        };

        let (account_number, sequence) =
            CosmosHandler::get_account(address, lcd_addr.clone()).await?;
        let bytes_to_sign = tx.get_sign_bytes(chain_id, account_number, sequence);
        let signature_block = StdSignature::sign(signer, bytes_to_sign);
        tx.signatures.push(signature_block.clone());
        let wrapped_tx = serde_json::json!({"tx": &tx, "mode":"block", "account_number": &account_number.to_string(), "sequence": &sequence.to_string()});

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
            error!(
                "Tx failed log: {:?} at height: {:?}",
                tx_response.raw_log, tx_response.height
            );
            return Err(format!("Tx failed, response from node: {:?}", tx_response));
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
        let response: AccountQueryResponse = serde_json::from_str(&acc_rstr).map_err(to_string)?;
        Ok((
            response.result.value.account_number,
            response.result.value.sequence,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::CosmosHandler;
    use signature::Signer;
    use k256::ecdsa::Signature;
    use k256::elliptic_curve::SecretKey;
    use k256::EncodedPoint as Secp256k1;

    const EXAMPLE_SEED: &str = "sunny source soul allow brave luggage mandate metal worth state vapor couple butter retreat solid drift cargo alley degree junk bean price element easy";

    #[test]
    fn test_signer_from_seed() {
        let (signer, pk, addr) = CosmosHandler::signer_from_seed(EXAMPLE_SEED.to_string()).unwrap();
        let sig: Signature = signer.sign(&"test".as_bytes());

        assert_eq!(addr, "cosmos1xccsl78jz98ydsfahrnluxefyvcnavuy4g3wd5");
        assert_eq!(pk.to_hex(), "EB5AE9872102B13C4ABBF9BEBCBFD0C99F0C9D130FDA36D5DFE5E3D93A182CB46BB93A27D732");
        assert_eq!(tendermint_light_client::PublicKey::from(Secp256k1::from_secret_key(&SecretKey::from(&signer), true)), pk);
        assert_eq!(
            hex::encode(sig.as_ref()),
            "fe740779fefacfaacebc41973c20cdb827378f92ae3ca66422dfbb0740e962cc1aed2452c265a6aeeccbd0100d03f6b1c7052e8f17a77f5607dbf95f08e62b1c"
        );
    }
}
