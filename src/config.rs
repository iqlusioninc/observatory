//! Observatory Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};
use tendermint::{account, chain};

/// Observatory Configuration
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryConfig {
    /// Chain configurations.
    #[serde(rename = "chain")]
    pub chains: Vec<ChainConfig>
}

/// Observatory Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainConfig {
    /// Chain ID
    pub id: chain::Id,

    /// Validator Addr
    pub validator_addr: account::Id,

    /// RPC URLs
    pub rpc_urls: Vec<String>,
}