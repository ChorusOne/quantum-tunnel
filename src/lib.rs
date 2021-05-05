//! QuantumTunnel
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]


#[macro_use]
extern crate cfg_if;

pub mod application;
pub mod commands;
pub mod config;
mod cosmos;
pub mod error;
pub mod prelude;
mod utils;

cfg_if! {
    if #[cfg(feature = "substrate")] {
        #[macro_use]
        extern crate substrate_subxt_proc_macro;

        mod substrate;
    } else if #[cfg(feature = "celo")] {
        mod celo;
    }
}
