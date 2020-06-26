//! Main entry point for QuantumTunnel

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use quantum_tunnel::application::APPLICATION;

/// Boot QuantumTunnel
fn main() {
    abscissa_core::boot(&APPLICATION);
}
