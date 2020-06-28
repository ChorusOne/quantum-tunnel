//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;

use crate::config::QuantumTunnelConfig;
use abscissa_core::{config, Command, FrameworkError, Options, Runnable};
use futures::{future::{join, join_all}, channel::mpsc::{UnboundedSender as Sender, UnboundedReceiver as Receiver, unbounded}};
use crate::cosmos::{Handler as CosmosHandler, types::TMHeader};
use crate::substrate::types::SignedBlockWithAuthoritySet;
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

        let cosmos_chan: (Sender<TMHeader>, Receiver<TMHeader>) = unbounded();
        let substrate_chan: (Sender<SignedBlockWithAuthoritySet>, Receiver<SignedBlockWithAuthoritySet>) = unbounded();


        let mut cosmos_handler = CosmosHandler::new(config.cosmos.clone()).await.unwrap();


        let mut threads = vec![];
        threads.push(spawn(async move {
            cosmos_handler.recv_handler(cosmos_chan.0).await;
            // if let Err(e) =  {
            //     println!("an error occurred; error = {:?}", e);
            // }
        }));

        // catch interrupt here, and terminate threads.
        join_all(threads).await;
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
