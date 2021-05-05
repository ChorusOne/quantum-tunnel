//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;

use crate::config::{CosmosChainConfig, QuantumTunnelConfig, SubstrateChainConfig, CeloChainConfig};
use crate::cosmos::Handler as CosmosHandler;
use abscissa_core::error::Context;
use abscissa_core::{config, Command, FrameworkError, FrameworkErrorKind, Options, Runnable};
use crossbeam_channel::unbounded;
use std::env;

cfg_if! {
    if #[cfg(feature = "substrate")] {
        use crate::substrate::types::SignedBlockWithAuthoritySet;
        use crate::substrate::Handler as SubstrateHandler;
        use crate::cosmos::types::TMHeader;

        async fn recv_handler(
            config: &QuantumTunnelConfig,
            other_chain_chan_tx: crossbeam_channel::Sender<SignedBlockWithAuthoritySet>,
            simulation_monitoring_rx: crossbeam_channel::Receiver<(bool, u64)>
        ) -> Result<(), String> {
            SubstrateHandler::recv_handler(
                config.substrate.clone().unwrap(),
                other_chain_chan_tx,
                simulation_monitoring_rx,
            ).await
        }

        async fn send_handler(
            config: &QuantumTunnelConfig,
            client_id: Option<String>,
            cosmos_chan_rx: crossbeam_channel::Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
            simulation_monitoring_tx: crossbeam_channel::Sender<(bool, u64)>,
        ) -> Result<(), String> {
            SubstrateHandler::send_handler(
                config.substrate.clone().unwrap(),
                client_id,
                cosmos_chan_rx,
                simulation_monitoring_tx.clone()
            ).await
        }

    } else if #[cfg(feature = "celo")] {
        use crate::celo::types::msg::CeloWrappedHeader;
        use crate::celo::Handler as CeloHandler;
        use crate::cosmos::types::TMHeader;

        async fn recv_handler(
            config: &QuantumTunnelConfig,
            other_chain_chan_tx: crossbeam_channel::Sender<CeloWrappedHeader>,
            simulation_monitoring_rx: crossbeam_channel::Receiver<(bool, u64)>
        ) -> Result<(), String> {
            CeloHandler::recv_handler(
                config.celo.clone().unwrap(),
                other_chain_chan_tx,
                simulation_monitoring_rx,
            ).await
        }

        async fn send_handler(
            config: &QuantumTunnelConfig,
            client_id: Option<String>,
            cosmos_chan_rx: crossbeam_channel::Receiver<(TMHeader, Vec<tendermint::validator::Info>)>,
            simulation_monitoring_tx: crossbeam_channel::Sender<(bool, u64)>,
        ) -> Result<(), String> {
            CeloHandler::send_handler(
                config.celo.clone().unwrap(),
                client_id,
                cosmos_chan_rx,
                simulation_monitoring_tx.clone()
            ).await
        }
    }
}

/// `start` subcommand
///
/// The `Options` proc macro generates an option parser based on the struct
/// definition, and is defined in the `gumdrop` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/gumdrop/>
#[derive(Command, Debug, Options)]
pub struct StartCmd {
    /// To whom are we saying hello?
    #[options(free)]
    cosmos_chain_id: String,
    cosmos_client: String,
    substrate_client: String,
    celo_client: String,
}

impl Runnable for StartCmd {
    /// Start the application.
    #[tokio::main]
    async fn run(&self) {
        let config = app_config();
        let (cosmos_chan_tx, cosmos_chan_rx) = unbounded();
        let (other_chain_chan_tx, other_chain_chan_rx) = unbounded();

        // Simulation monitoring channels are used to send information about consumed blocks to
        // simulation handler which enables simulation handler to detect successful or failure
        // simulation.
        // Sender goes to recv_handler and Receiver goes to send_handler
        // This channels will be ignored if it is live chain
        let (simulation_monitoring_tx, simulation_monitoring_rx) = unbounded();

        let mut cosmos_client_id = None;
        if !self.cosmos_client.is_empty() {
            cosmos_client_id = Some(self.cosmos_client.clone());
        }

        let mut other_client_id = None;
        if !self.substrate_client.is_empty() {
            other_client_id = Some(self.substrate_client.clone());
        } else if !self.celo_client.is_empty() {
            other_client_id = Some(self.celo_client.clone());
        }

        tokio::select! {
            res = CosmosHandler::recv_handler(
                config.cosmos.clone(),
                cosmos_chan_tx,
                simulation_monitoring_rx.clone()
            ) => {
                if res.is_err() {
                    panic!(format!("Error occurred while receiving data from Cosmos chain: {}", res.err().unwrap()));
                }
            },
            res = recv_handler(
                &config,
                other_chain_chan_tx,
                simulation_monitoring_rx.clone()
            ) => {
                if res.is_err() {
                    panic!(format!("Error occurred while receiving data from other chain: {}", res.err().unwrap()));
                }
            },
            res = CosmosHandler::send_handler(
                config.cosmos.clone(),
                cosmos_client_id,
                other_chain_chan_rx,
                simulation_monitoring_tx.clone()
            ) => {
                if res.is_err() {
                    panic!(format!("Error occurred while sending data to cosmos chain: {}", res.err().unwrap()));
                }
            },
            res = send_handler(
                &config,
                other_client_id,
                cosmos_chan_rx,
                simulation_monitoring_tx.clone()
            ) => {
                if res.is_err() {
                    panic!(format!("Error occurred while sending data to other chain: {}", res.err().unwrap()));
                }
            }
        }
    }
}

impl config::Override<QuantumTunnelConfig> for StartCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(
        &self,
        mut config: QuantumTunnelConfig,
    ) -> Result<QuantumTunnelConfig, FrameworkError> {
        let mut is_live = false;

        // Only one counter-chain should be present in configuration
        let chain_presence = vec![
            config.substrate.is_some(),
            config.celo.is_some()
        ];
        if chain_presence.iter().filter(|&v| *v).count() != 1 {
            return Err(FrameworkError::from(Context::new(
                        FrameworkErrorKind::ConfigError,
                        None,
            )))
        }

        match (&mut config.cosmos, &mut config.substrate, &mut config.celo) {
            // Both configuration cannot be simulation at the same time
            (CosmosChainConfig::Simulation(_), Some(SubstrateChainConfig::Simulation(_)), _) => {
                return Err(FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    None,
                )))
            }
            (CosmosChainConfig::Simulation(_), _, Some(CeloChainConfig::Simulation(_))) => {
                return Err(FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    None,
                )))
            }
            (CosmosChainConfig::Real(_), Some(SubstrateChainConfig::Real(_)), _) => {
                is_live = true;
            }
            (CosmosChainConfig::Real(_), _, Some(CeloChainConfig::Real(_))) => {
                is_live = true;
            }
            _ => {}
        }

        if let CosmosChainConfig::Real(ref mut cfg) = config.cosmos {
            if !self.cosmos_chain_id.is_empty() {
                cfg.chain_id = self.cosmos_chain_id.clone();
            }

            // Let's read environment variables to get seed data.
            cfg.signer_seed = env::var("COSMOS_SIGNER_SEED").map_err(|e| {
                FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    Some(Box::new(e)),
                ))
            })?;

            cfg.is_other_side_simulation = !is_live;
        }

        if let Some(SubstrateChainConfig::Real(ref mut cfg)) = config.substrate {
            // Let's read environment variables to get seed data.
            cfg.signer_seed = env::var("SUBSTRATE_SIGNER_SEED").map_err(|e| {
                FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    Some(Box::new(e)),
                ))
            })?;

            cfg.is_other_side_simulation = !is_live;
        }

        if let Some(CeloChainConfig::Real(ref mut cfg)) = config.celo {
            // Let's read environment variables to get seed data.
            cfg.signer_seed = env::var("CELO_SIGNER_SEED").map_err(|e| {
                FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    Some(Box::new(e)),
                ))
            })?;

            cfg.is_other_side_simulation = !is_live;
        }

        Ok(config)
    }
}
