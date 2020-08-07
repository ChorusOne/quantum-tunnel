//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;

use crate::config::{CosmosChainConfig, QuantumTunnelConfig, SubstrateChainConfig};
use crate::cosmos::Handler as CosmosHandler;
use crate::substrate::Handler as SubstrateHandler;
use abscissa_core::{config, Command, FrameworkError, FrameworkErrorKind, Options, Runnable};
use crossbeam_channel::unbounded;
use futures::future::try_join_all;

use abscissa_core::error::Context;
use std::env;
use tokio::spawn;
use tokio::sync::oneshot;

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
}

impl Runnable for StartCmd {
    /// Start the application.
    #[tokio::main]
    async fn run(&self) {
        let config = app_config();
        let (cosmos_chan_tx, cosmos_chan_rx) = unbounded();
        let (substrate_chan_tx, substrate_chan_rx) = unbounded();

        let mut cosmos_client = None;
        if !self.cosmos_client.is_empty() {
            cosmos_client = Some(self.cosmos_client.clone());
        }

        let mut substrate_client = None;
        if !self.substrate_client.is_empty() {
            substrate_client = Some(self.substrate_client.clone());
        }

        tokio::select! {
            res = CosmosHandler::recv_handler(
                config.cosmos.clone(),
                cosmos_chan_tx,
            ) => {
                match res {
                    Ok(_) => {
                        // This should never happen
                    },
                    Err(msg) => panic!(format!("Error occurred while receiving data from Cosmos chain: {}", msg)),
                }
            },
            res = SubstrateHandler::recv_handler(
                config.substrate.clone(),
                substrate_chan_tx,
            ) => {
                match res {
                    Ok(_) => {
                        // This should never happen
                    },
                    Err(msg) => panic!(format!("Error occurred while receiving data from Substrate chain: {}", msg)),
                }
            },
            res = SubstrateHandler::send_handler(
                config.substrate.clone(),
                substrate_client,
                cosmos_chan_rx,
            ) => {
                match res {
                    Ok(_) => {
                        // This should never happen
                    },
                    Err(msg) => panic!(format!("Error occurred while sending data to Substrate chain: {}", msg)),
                }
            },
            res = CosmosHandler::send_handler(
                config.cosmos.clone(),
                cosmos_client,
                substrate_chan_rx,
            ) => {
                match res {
                    Ok(_) => {
                        // This should never happen
                    },
                    Err(msg) => panic!(format!("Error occurred while sending data to cosmos chain: {}", msg)),
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
        match (&mut config.cosmos, &mut config.substrate) {
            // Both configuration cannot be simulation at the same time
            (CosmosChainConfig::Simulation(_), SubstrateChainConfig::Simulation(_)) => {
                return Err(FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    None,
                )))
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
        }

        if let SubstrateChainConfig::Real(ref mut cfg) = config.substrate {
            // Let's read environment variables to get seed data.
            cfg.signer_seed = env::var("SUBSTRATE_SIGNER_SEED").map_err(|e| {
                FrameworkError::from(Context::new(
                    FrameworkErrorKind::ConfigError,
                    Some(Box::new(e)),
                ))
            })?;
        }

        Ok(config)
    }
}
