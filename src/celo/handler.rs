use crate::celo::sync::SyncClient;
use crate::celo::types::msg::CeloWrappedHeader;
use crate::config::{CeloChainConfig, CeloConfig};
use crate::cosmos::types::TMHeader;
use crate::utils::to_string;
use celo_types::{
    client::LightClientState, consensus::LightConsensusState, hash_header,
    header::Header as CeloHeader, istanbul::IstanbulExtra,
};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use ethereum_types::{Address, U256};
use futures::{SinkExt, StreamExt};
use log::*;
use serde_json::{from_str, Value};
use std::path::Path;
use std::string::ToString;
use tokio_tungstenite::connect_async;

pub struct CeloHandler {}
impl CeloHandler {
    /// Receive handler entrypoint
    /// Branches to different internal methods depending upon whether
    /// configuration is `Real` or `Simulation`
    pub async fn recv_handler(
        cfg: CeloChainConfig,
        outchan: Sender<CeloWrappedHeader>,
        monitoring_inchan: Receiver<(bool, u64)>,
    ) -> Result<(), String> {
        match cfg {
            CeloChainConfig::Real(cfg) => Self::chain_recv_handler(cfg, outchan).await,
            CeloChainConfig::Simulation(cfg) => {
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
        outchan: Sender<CeloWrappedHeader>,
        monitoring_inchan: Receiver<(bool, u64)>,
    ) -> Result<(), String> {
        let simulation_data =
            std::fs::read_to_string(Path::new(test_file.as_str())).map_err(to_string)?;
        let stringified_headers: Vec<&str> = simulation_data.split("\n\n").collect();
        let number_of_simulated_headers = stringified_headers.len();
        for str in stringified_headers {
            let payload = from_str(str).map_err(to_string)?;

            outchan.try_send(payload).map_err(to_string)?;
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
                        "Celo headers simulated successfully. Ingested headers till height: {}",
                        successfully_ingested_till
                    );
                }
                break;
            } else {
                info!(
                    "Cosmos light client has successfully ingested header at: {}",
                    successfully_ingested_till
                );
            }
        }
        Ok(())
    }

    /// Subscribes to new blocks from Websocket, and pushes CeloHeader objects into the Channel.
    pub async fn chain_recv_handler(
        cfg: CeloConfig,
        outchan: Sender<CeloWrappedHeader>,
    ) -> Result<(), String> {
        // Fetch relevant configuration
        let state_config = LightClientState {
            chain_id: 1,

            counterparty_address: Address::default(),
            commitment_map_position: U256::default(),
            next_sequence_rx_map_position: U256::default(),

            epoch_size: cfg.epoch_size,
            allowed_clock_skew: parse_duration::parse(cfg.max_clock_drift.as_str())
                .map_err(to_string)?
                .as_secs(),
            upgrade_path: Vec::new(),

            verify_epoch_headers: cfg.verify_epoch_headers,
            verify_non_epoch_headers: cfg.verify_non_epoch_headers,
            verify_header_timestamp: cfg.verify_header_timestamp,

            allow_update_after_expiry: cfg.allow_update_after_expiry,
            allow_update_after_misbehavior: cfg.allow_update_after_expiry,
            trusting_period: parse_duration::parse(cfg.trusting_period.as_str())
                .map_err(to_string)?
                .as_secs(),
        };

        // Fetch initial state (validators set) from remote full-node
        info!("fetching initial state from: {}", cfg.rpc_addr.clone());
        let initial_header = get_initial_header(cfg.rpc_addr.clone(), state_config)
            .await
            .map_err(to_string)?;

        // Send the initial header / state before others, so that contract is initialized properly
        info!(
            "send initial header at height: {}",
            initial_header.initial_consensus_state.number
        );
        outchan
            .try_send(initial_header.clone())
            .map_err(to_string)?;

        // Subscribe to recieve new blocks
        let (mut socket, _) = connect_async(&cfg.ws_addr).await.map_err(to_string)?;
        info!("connected websocket to {:?}", &cfg.ws_addr);
        let subscribe_message = tokio_tungstenite::tungstenite::Message::Text(
            r#"{"id": 0, "method": "eth_subscribe", "params": ["newHeads"]}"#.to_string(),
        );
        socket.send(subscribe_message).await.map_err(to_string)?;

        let initial_consensus_state = initial_header.initial_consensus_state;
        let initial_client_state = initial_header.initial_client_state;

        async fn process_msg(
            msg: tokio_tungstenite::tungstenite::Message,
            initial_consensus_state: LightConsensusState,
            initial_client_state: LightClientState,
        ) -> Result<Option<CeloWrappedHeader>, String> {
            let msgtext = msg.to_text().map_err(to_string)?;
            let json = from_str::<Value>(msgtext).map_err(to_string)?;
            let raw_header = json["params"]["result"].to_string();
            let current_header: CeloHeader =
                serde_json::from_slice(&raw_header.as_bytes()).map_err(to_string)?;

            if current_header.number.as_u64() < initial_consensus_state.number {
                info!("recieved header height is lower than initial state height, skipping");
                return Ok(None);
            }

            let header: CeloWrappedHeader = CeloWrappedHeader {
                header: current_header,
                initial_consensus_state,
                initial_client_state,
            };

            Ok(Some(header))
        }

        while let Some(msg) = socket.next().await {
            if let Ok(msg) = msg {
                info!("Received message from celo chain: {:?}", msg);
                match process_msg(
                    msg.clone(),
                    initial_consensus_state.clone(),
                    initial_client_state.clone(),
                )
                .await
                {
                    Ok(maybe_header) => match maybe_header {
                        Some(celo_header) => outchan.try_send(celo_header).map_err(to_string)?,
                        None => {}
                    },
                    Err(err) => error!("Error: {}", err),
                }
            }
        }

        Ok(())
    }

    /// Send handler entrypoint
    /// Branches to different internal methods depending upon whether
    /// configuration is `Real` or `Simulation`
    /// If other side is simulation, some additional bookkeeping is done to
    /// make sure `simulation_recv_handler` gets accurate data.
    pub async fn send_handler(
        cfg: CeloChainConfig,
        client_id: Option<String>,
        inchan: Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> {
        match cfg {
            CeloChainConfig::Real(cfg) => {
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
                        error!("Error occurred while trying to send simulated cosmos data to celo chain: {}", result.err().unwrap());
                    }
                    // This gives simulation_recv_handler time to print result and then exit.
                    futures::future::pending::<()>().await;
                    Ok(())
                } else {
                    Self::chain_send_handler(cfg, client_id, inchan, monitoring_outchan).await
                }
            }
            // If we are running simulation, we cannot ingest any headers.
            CeloChainConfig::Simulation(_cfg) => {
                loop {
                    let result = inchan.try_recv();
                    if result.is_err() {
                        match result.err().unwrap() {
                            TryRecvError::Disconnected => {
                                return Err(
                                    "cosmos chain-data channel's input end is disconnected."
                                        .to_string(),
                                );
                            }
                            _ => {}
                        }
                    }
                    // Compulsory delay of 1 second to not enter in busy loop.
                    tokio::time::sleep(core::time::Duration::new(1, 0)).await;
                }
            }
        }
    }

    /// Transforms header data received from opposite chain to
    /// light client payload and sends it to tendermint light client running in
    /// substrate chain.
    /// If client id is not passed, first payload sent would be for creating the client.
    pub async fn chain_send_handler(
        cfg: CeloConfig,
        _client_id: Option<String>,
        inchan: Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
        monitoring_outchan: Sender<(bool, u64)>,
    ) -> Result<(), String> {
        let mut new_client = false;
        // TODO: We don't have a CosmosClient (tendermint-light-client) running on the CeloBlockchain yet,
        // so this is just a stub method.
        //
        // Cosmos[CeloLightWasm] in: Celo out: Cosmos <---> in: Cosmos, out: Celo Celo[TendermintLight]
        // ^^ we implement this                             !!^^ not this

        loop {
            let result = inchan.try_recv();
            let msg = if result.is_err() {
                match result.err().unwrap() {
                    TryRecvError::Disconnected => {
                        return Err(
                            "cosmos chain-data channel's input end is disconnected.".to_string()
                        );
                    }
                    _ => {
                        warn!("Did not receive any data from Cosmos chain-data channel. Retrying in a second ...");
                        tokio::time::sleep(core::time::Duration::new(1, 0)).await;
                        continue;
                    }
                }
            } else {
                result.unwrap()
            };
            let current_height = msg.0.signed_header.header.height.value();

            if new_client {
                new_client = false;
                info!("Created Cosmos light client");
            } else {
                info!(
                    "{}",
                    format!(
                        "Updating Cosmos light client with block at height: {}",
                        current_height
                    )
                );
                info!("Updated Cosmos light client");
            }
            if cfg.is_other_side_simulation {
                monitoring_outchan
                    .try_send((false, current_height))
                    .map_err(to_string)?;
            }
        }
    }
}

pub async fn get_initial_header(
    addr: String,
    state_config: LightClientState,
) -> Result<CeloWrappedHeader, String> {
    info!("InitialState: Setting up client");
    let relayer = SyncClient::new(addr.clone());

    info!("InitialState: Fetch last block header");
    let current_block_header: CeloHeader = relayer
        .get_block_header_by_number("latest")
        .await
        .map_err(to_string)?;
    let _extra = celo_types::extract_istanbul_extra(&current_block_header).map_err(to_string)?;
    let last_block_num = current_block_header.number.as_u64();
    let last_block_num_hex: String = format!("0x{:x}", last_block_num);

    info!(
        "InitialState: Fetch current validator set for block: {}",
        last_block_num_hex
    );
    let validators = relayer
        .get_current_validators(&last_block_num_hex)
        .await
        .map_err(to_string)?;

    Ok(CeloWrappedHeader {
        header: current_block_header.clone(),
        initial_client_state: state_config,
        initial_consensus_state: LightConsensusState {
            validators,
            number: last_block_num,
            hash: hash_header(&current_block_header),
        },
    })
}
