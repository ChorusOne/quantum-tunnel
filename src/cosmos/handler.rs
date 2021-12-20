use crate::config::{CosmosChainConfig, CosmosConfig};
use crate::cosmos::crypto::{privkey_from_seed, seed_from_mnemonic};
use crate::cosmos::types::simulation::Message as SimMessage;
use crate::cosmos::types::{
    DecCoin,
    TMHeader,
    std_sign_bytes
};
use crate::cosmos::types::WasmHeader;
use crate::error::ErrorKind;
use crate::error::ErrorKind::{MalformedResponse, UnexpectedPayload};
use crate::utils::{to_string, prost_serialize};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use futures::try_join;
use log::*;
use k256::{elliptic_curve::SecretKey, ecdsa::{SigningKey, Signature}};
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
use prost_types::Any;
use signature::Signer;
use prost::Message as ProstMessage;

use crate::cosmos::proto::cosmos::tx::v1beta1::{
    Tx, AuthInfo, Fee, ModeInfo, SignerInfo, TxBody, BroadcastTxRequest,
    service_client::ServiceClient
};
use crate::cosmos::proto::cosmos::tx::v1beta1::mode_info::{Single, Sum};


use crate::cosmos::proto::{
    cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest, query_client::QueryClient},
};

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
            let payload: SimMessage = serde_json::from_str(str).map_err(to_string)?;
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
                        tokio::time::sleep(core::time::Duration::new(1, 0)).await;
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
        info!("opening websocket to to {:?}", tm_addr);
        let (mut client, driver) = WebSocketClient::new(tm_addr.clone())
            .await
            .map_err(to_string)?;

        let driver_handle = tokio::spawn(async move { driver.run().await });

        info!("connected websocket to {:?}", tm_addr);
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
    pub async fn send_handler<T>(
        cfg: CosmosChainConfig,
        client_id: Option<String>,
        inchan: Receiver<T>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> where T: WasmHeader {
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
                                    format!("{} chain-data channel's input end is disconnected.", T::chain_name())
                                );
                            }
                            _ => {}
                        }
                    }
                    // Compulsory delay of 1 second to prevent busy loop.
                    tokio::time::sleep(core::time::Duration::new(1, 0)).await;
                }
            }
        }
    }

    /// Transforms header data received from opposite chain to
    /// light client payload and sends it to substrate light client running in
    /// cosmos chain.
    /// If client id is not passed, first payload sent would be for creating the client.
    pub async fn chain_send_handler<T>(
        cfg: CosmosConfig,
        client_id: Option<String>,
        inchan: Receiver<T>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> where T: WasmHeader {
        let mut new_client = true;
        let mut id = if !client_id.is_some() {
            new_client = true;
            String::default()
        } else {
            client_id.unwrap()
        };

        loop {
            let result = inchan.try_recv();
            let msg = if result.is_err() {
                match result.err().unwrap() {
                    TryRecvError::Disconnected => {
                        return Err(
                            format!("{} chain-data channel's input end is disconnected.", T::chain_name())
                        );
                    }
                    _ => {
                        warn!("Did not receive any data from {} chain-data channel. Retrying in a second ...", T::chain_name());
                        tokio::time::sleep(core::time::Duration::new(1, 0)).await;
                        continue;
                    }
                }
            } else {
                result.unwrap()
            };

            let current_height = msg.height();

            if new_client {
                new_client = false;
                id = CosmosHandler::create_client::<T>(&cfg, msg).await?;
            } else {
                CosmosHandler::update_client::<T>(&cfg, msg, id.clone()).await?;
            }

            if cfg.is_other_side_simulation {
                monitoring_outchan
                    .try_send((false, current_height))
                    .map_err(to_string)?;
            }
        }
    }

    // celo daeamon -> relayer -> celo light client wasm thingy running in cosmos
    pub async fn create_client<T>(
        cfg: &CosmosConfig,
        header: T,
    ) -> Result<String, String> where T: WasmHeader {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.signer_seed.to_owned()).map_err(to_string)?;

        let m = header.to_wasm_create_msg(&cfg, address.to_owned()).map_err(to_string)?;
        let f = Fee {
            amount: vec![DecCoin::from(cfg.gas_price.to_owned()).mul(cfg.gas as f64).to_coin()],
            gas_limit: cfg.gas,
            payer: "".to_string(),
            granter: "".to_string(),
        };

        let retval = CosmosHandler::submit_tx(
            m,
            f,
            "".to_owned(),
            signer,
            address,
            cfg.chain_id.clone(),
            cfg.grpc_addr.clone(),
            true
        )
        .await
        .map_err(to_string)?;
        info!("Celo light client creation TxHash: {:?}", retval.0);
        Ok(retval.1)
    }

    pub async fn update_client<T>(
        cfg: &CosmosConfig,
        header: T,
        client_id: String,
    ) -> Result<String, String> where T: WasmHeader {
        let (signer, _, address) =
            CosmosHandler::signer_from_seed(cfg.signer_seed.to_owned()).map_err(to_string)?;

        let msgs = header.to_wasm_update_msg(address.to_owned(), client_id).map_err(to_string)?;

        let txfee = Fee {
            amount: vec![DecCoin::from(cfg.gas_price.to_owned()).mul(cfg.gas as f64).to_coin()],
            gas_limit: cfg.gas,
            payer: "".to_string(),
            granter: "".to_string(),
        };

        // this is okay (universal call)
        let retval = CosmosHandler::submit_tx(
            msgs,
            txfee,
            "".to_owned(),
            signer,
            address,
            cfg.chain_id.clone(),
            cfg.grpc_addr.clone(),
            false
        )
        .await
        .map_err(to_string)?;
        info!("{} light client updation TxHash: {:?}", T::chain_name(), retval.0);
        Ok(retval.0)
    }

    async fn submit_tx(
        msgs: Vec<Any>,
        fee: Fee,
        memo: String,
        signer: SigningKey,
        address: String,
        chain_id: String,
        grpc_addr: String,
        find_client_id: bool,
    ) -> Result<(String, String), String> {
        // fetch cosmos account number
        let (account_number, sequence) =
            CosmosHandler::get_account(address, grpc_addr.clone()).await?;

        // perpare public key
        let secret_key = SecretKey::from(&signer);
        let public_key = Secp256k1::from_secret_key(&secret_key, true);
        let pk_any = Any {
            type_url: "/cosmos.crypto.secp256k1.PubKey".to_string(),
            value: prost_serialize(&public_key.as_bytes().to_vec()).map_err(to_string)?,
        };

        // prepare Tx segments
        let single = Single { mode: 1 };
        let sum_single = Some(Sum::Single(single));
        let mode = Some(ModeInfo { sum: sum_single });

        let tx_body = TxBody {
            messages: msgs,
            memo,
            timeout_height: 0,
            extension_options: Vec::<Any>::new(),
            non_critical_extension_options: Vec::<Any>::new(),
        };

        let auth_info = AuthInfo {
            signer_infos: vec![
                SignerInfo {
                    public_key: Some(pk_any),
                    mode_info: mode,
                    sequence: sequence,
                }
            ],
            fee: Some(fee),
        };

        // create transaction signature
        let bytes_to_sign = std_sign_bytes(&tx_body, &auth_info, chain_id, account_number).map_err(to_string)?;
        let signature: Signature = signer.sign(bytes_to_sign.as_slice());

        let tx = Tx {
            body: Some(tx_body),
            auth_info: Some(auth_info),
            signatures: vec![signature.as_ref().to_vec()],
        };

        let mut client = ServiceClient::connect(grpc_addr).await.map_err(to_string)?;

        let request = tonic::Request::new(BroadcastTxRequest {
            tx_bytes: prost_serialize(&tx).map_err(to_string)?,
            mode: 1,
        });

        let response = client.broadcast_tx(request).await.map_err(to_string)?;
        let tx_response = response.into_inner().tx_response.ok_or("failed to get tx_response")?;

        if tx_response.code != 0 {
            error!(
                "Tx failed log: {:?} at height: {:?}",
                tx_response.raw_log, tx_response.height
            );
            return Err(format!("Tx failed, response from node: {:?}", tx_response));
        };

        let client_id = if find_client_id {
            let event = tx_response.logs
                .get(0).ok_or("empty event log")?
                .events
                .iter()
                .find(|&event| event.r#type == "create_client")
                .ok_or("can't find create_client event in the log")?;

            let attribute = event.attributes
                .iter()
                .find(|attr| attr.key == "client_id")
                .ok_or("unable to find client_id attribute")?;

            attribute.value.clone()
        } else {
            String::default()
        };

        Ok((tx_response.txhash, client_id))
    }

    async fn get_account(account: String, grpc_addr: String) -> Result<(u64, u64), String> {
        let mut client = QueryClient::connect(grpc_addr).await.map_err(to_string)?;

        let request = tonic::Request::new(QueryAccountRequest {
            address: account
        });

        let response = client.account(request).await.map_err(to_string)?;
        let account: BaseAccount = match response.into_inner().account {
            Some(acc) => ProstMessage::decode(acc.value.as_slice()).map_err(to_string)?,
            None => return Err("failed to extract account from response".to_string()),
        };

        Ok((
            account.account_number,
            account.sequence
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
