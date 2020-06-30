//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;
use futures::future::join_all;
use crate::config::QuantumTunnelConfig;
use crate::cosmos::{types::TMHeader, Handler as CosmosHandler};
use crate::substrate::{types::SignedBlockWithAuthoritySet, Handler as SubstrateHandler};
use crossbeam_channel::{unbounded, Sender, Receiver};
use abscissa_core::{config, Command, FrameworkError, Options, Runnable};
use futures::{FutureExt, TryFutureExt};

use tokio::spawn;

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
}

impl Runnable for StartCmd {
    /// Start the application.
    #[tokio::main]
    async fn run(&self) {
        let config = app_config();
        let (cosmos_chan_tx, cosmos_chan_rx) = unbounded();
        let (substrate_chan_tx , substrate_chan_rx) = unbounded();

        let mut tasks = vec![];
        tasks.push(spawn(CosmosHandler::recv_handler(config.cosmos.clone(), cosmos_chan_tx)));
        tasks.push(spawn(SubstrateHandler::recv_handler(config.substrate.clone(), substrate_chan_tx)));
        tasks.push(spawn(SubstrateHandler::send_handler(config.substrate.clone(), cosmos_chan_rx)));
        tasks.push(spawn(CosmosHandler::send_handler(config.cosmos.clone(), "xxxxxxxxxx".to_string(), substrate_chan_rx)));

        // catch interrupt here, and terminate threads.

        join_all(tasks).await.iter().for_each(|join_result| {
            if join_result.is_err() {
                panic!("Could not join the tasks. Error: {:?}", join_result.as_ref().err().unwrap());
            }
            let task_result = join_result.as_ref().unwrap();
            if task_result.is_err() {
                panic!("Task errored out. Error: {:?}", task_result.as_ref().err().unwrap());
            }
        });
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
        if !self.cosmos_chain_id.is_empty() {
            config.cosmos.chain_id = self.cosmos_chain_id.clone();
        }

        Ok(config)
    }
}
