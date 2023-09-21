//! Observatory
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

pub mod application;
mod chain_monitor;
mod chain_state;
mod client_manager;
pub mod commands;
pub mod config;
pub mod error;
pub mod prelude;

/// URL type.
// TODO(tarcieri): use `url` crate?
pub type Url = String;
