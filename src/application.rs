//! QuantumTunnel Abscissa Application

use crate::{commands::QuantumTunnelCmd, config::QuantumTunnelConfig};
use abscissa_core::error::framework::FrameworkErrorKind::{ConfigError, IoError, PathError};
use abscissa_core::path::AbsPathBuf;
use abscissa_core::{
    application::{self, AppCell},
    config, trace, Application, EntryPoint, FrameworkError, StandardPaths,
};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Application state
pub static APPLICATION: AppCell<QuantumTunnelApp> = AppCell::new();

/// Obtain a read-only (multi-reader) lock on the application state.
///
/// Panics if the application state has not been initialized.
pub fn app_reader() -> application::lock::Reader<QuantumTunnelApp> {
    APPLICATION.read()
}

/// Obtain an exclusive mutable lock on the application state.
pub fn app_writer() -> application::lock::Writer<QuantumTunnelApp> {
    APPLICATION.write()
}

/// Obtain a read-only (multi-reader) lock on the application configuration.
///
/// Panics if the application configuration has not been loaded.
pub fn app_config() -> config::Reader<QuantumTunnelApp> {
    config::Reader::new(&APPLICATION)
}

/// QuantumTunnel Application
#[derive(Debug)]
pub struct QuantumTunnelApp {
    /// Application configuration.
    config: Option<QuantumTunnelConfig>,

    /// Application state.
    state: application::State<Self>,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for QuantumTunnelApp {
    fn default() -> Self {
        Self {
            config: None,
            state: application::State::default(),
        }
    }
}

impl Application for QuantumTunnelApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint<QuantumTunnelCmd>;

    /// Application configuration.
    type Cfg = QuantumTunnelConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> &QuantumTunnelConfig {
        self.config.as_ref().expect("config not loaded")
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Borrow the application state mutably.
    fn state_mut(&mut self) -> &mut application::State<Self> {
        &mut self.state
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let components = self.framework_components(command)?;
        self.state.components.register(components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        self.state.components.after_config(&config)?;
        self.config = Some(config);
        Ok(())
    }

    fn load_config(&mut self, path: &Path) -> Result<Self::Cfg, FrameworkError> {
        let canonical_path = AbsPathBuf::canonicalize(path).map_err(|_e| {
            let path_error = PathError {
                name: Some(path.into()),
            };
            FrameworkError::from(ConfigError.context(path_error))
        })?;

        let mut file = File::open(AsRef::<Path>::as_ref(&canonical_path)).map_err(|e| {
            let io_error = IoError.context(e);
            let path_error = PathError {
                name: Some(canonical_path.into_path_buf()),
            }
            .context(io_error);
            ConfigError.context(path_error)
        })?;

        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;
        Ok(serde_json::from_str(&*json_string).map_err(|e| IoError.context(e))?)
    }

    /// Get tracing configuration from command-line options
    fn tracing_config(&self, command: &EntryPoint<QuantumTunnelCmd>) -> trace::Config {
        if command.verbose {
            trace::Config::verbose()
        } else {
            trace::Config::default()
        }
    }
}
