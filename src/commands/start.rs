//! `start` subcommand - example of how to write a subcommand

use crate::{
    chain_monitor::ChainMonitor,
    client_manager::ClientManager,
    config::{ChainConfig, ObservatoryConfig},
    pager::{monitor_pager_service, PagerBuffer, PagerRequest, PagerService},
    prelude::*,
};
use abscissa_core::{config, Command, FrameworkError, Runnable};
use futures::future;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower::{Service, ServiceExt};

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
        let alerting_interval = Duration::from_secs(120);
        let missing_blocks_threshold = 50;
        let recovered_after_threshold = 5;

        if config.chains.is_empty() {
            panic!("no chains configured (no 'observatory.toml'?)");
        }

        abscissa_tokio::run(&APP, async {
            let pager_service = tower::ServiceBuilder::new()
                .buffer(config.chains.len() * 2) // heuristic
                .service(PagerService::new(
                    missing_blocks_threshold,
                    recovered_after_threshold,
                ));

            let mut futures = Vec::new();

            for chain_config in &config.chains {
                futures.push(run_monitor(chain_config.clone(), pager_service.clone()).await);
            }

            futures.push(init_pager_monitor(alerting_interval, pager_service.clone()).await);

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

async fn run_monitor(config: ChainConfig, mut pager_service: PagerBuffer) -> JoinHandle<()> {
    tokio::spawn(async move {
        let chain_id = config.id;
        let validator_addr = config.validator_addr;
        let rpc_urls = config.rpc_urls;

        info!("[{chain_id}] monitoring signatures from {validator_addr}");

        let client_manager =
            ClientManager::new(rpc_urls).expect("couldn't initialize RPC client manager");

        let mut monitor = ChainMonitor::new(chain_id.clone(), client_manager).await;

        loop {
            monitor.fetch_next_block().await;

            let missed_blocks = monitor.missed_blocks(validator_addr);
            let recent_blocks = monitor.recent_blocks(validator_addr);

            pager_service
                .ready()
                .await
                .expect("PagerService not ready")
                .call(PagerRequest::Event {
                    chain_id: chain_id.clone(),
                    missed_blocks,
                    recent_blocks,
                })
                .await
                .expect("PagerService error");
        }
    })
}

async fn init_pager_monitor(
    alerting_interval: Duration,
    pager_service: PagerBuffer,
) -> JoinHandle<()> {
    tokio::spawn(
        async move { monitor_pager_service(alerting_interval, pager_service.clone()).await },
    )
}
