//! QuantumTunnel Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod start;
mod version;

use self::{start::StartCmd, version::VersionCmd};
use crate::config::QuantumTunnelConfig;
use abscissa_core::{
    config::Override, Command, Configurable, FrameworkError, Help, Options, Runnable,
};
use serde::Serialize;
use toml;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
/// QuantumTunnel Configuration Filename
pub const CONFIG_FILE: &str = "quantum_tunnel.toml";

/// QuantumTunnel Subcommands
#[derive(Command, Debug, Options, Runnable)]
pub enum QuantumTunnelCmd {
    /// The `help` subcommand
    // #[options(help = "get usage information")]
    // Help(Help<Self>),

    /// The `start` subcommand
    #[options(help = "start the application")]
    Start(StartCmd),

    /// The `version` subcommand
    #[options(help = "display version information")]
    Version(VersionCmd),

    // The `test-cosmos` subcommand
    // #[options(help = "interactively test the cosmos chain")]
    // CosmosTest(CosmosTestCmd),
    //
    // The `test-substrate` subcommand
    // #[options(help = "interactively test the substrate chain")]
    // SubstrateTest(SubstrateTestCmd),
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<QuantumTunnelConfig> for QuantumTunnelCmd {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Check if the config file exists, and if it does not, ignore it.
        // If you'd like for a missing configuration file to be a hard error
        // instead, always return `Some(CONFIG_FILE)` here.
        let filename = PathBuf::from(CONFIG_FILE);

        if !filename.exists() {
            let mut file = File::create(CONFIG_FILE).ok()?;
            let cfg = toml::to_string(&QuantumTunnelConfig::default()).unwrap();
            file.write_all(cfg.as_bytes()).ok()?;
        }
        Some(filename)

    }


    /// Apply changes to the config after it's been loaded, e.g. overriding

    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(
        &self,
        config: QuantumTunnelConfig,
    ) -> Result<QuantumTunnelConfig, FrameworkError> {
        match self {
            QuantumTunnelCmd::Start(cmd) => cmd.override_config(config),
            _ => Ok(config),
        }
    }
}
