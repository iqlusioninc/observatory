//! `start` subcommand - example of how to write a subcommand

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;

use crate::config::{ChainConfig, ObservatoryConfig};

use abscissa_core::{config, Command, FrameworkError, Runnable};

use crate::{chain_monitor::ChainMonitor, client_manager::ClientManager};
use futures::future;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// `start` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(clap::Parser, Command, Debug)]
pub struct StartCmd {
    /// To whom are we saying hello?
    recipient: Vec<String>,
}

impl Runnable for StartCmd {
    /// Start the application.
    fn run(&self) {
        let config = APP.config();
        dbg!(&config);
        abscissa_tokio::run(&APP, async {
            let mut futures = Vec::new();

            for chain_config in &config.chains {
                futures.push(run_monitor(chain_config.clone()).await);
            }

            future::join_all(futures).await;
        })
        .expect("Tokio runtime crashed");
    }
}

impl config::Override<ObservatoryConfig> for StartCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(
        &self,
        config: ObservatoryConfig,
    ) -> Result<ObservatoryConfig, FrameworkError> {
        Ok(config)
    }
}

async fn run_monitor(
    config: ChainConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let chain_id = config.id;
        let validator_addr = config.validator_addr;
        let rpc_urls = config.rpc_urls;

        info!("[{chain_id}] monitoring signatures from {validator_addr}");

        let client_manager =
            ClientManager::new(rpc_urls).expect("couldn't initialize RPC client manager");

        let mut monitor = ChainMonitor::new(chain_id.clone(), client_manager).await;
        let missed_blocks_threshold = 3;

        loop {
            monitor.fetch_next_block().await;

            let missed_blocks = monitor.missed_blocks(validator_addr);

            if missed_blocks > missed_blocks_threshold {
                warn!("{} missed {} blocks!", chain_id, missed_blocks);
            }
        }
    })
}
