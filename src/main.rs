mod chain_monitor;
mod chain_state;
mod client_manager;

use crate::{chain_monitor::ChainMonitor, client_manager::ClientManager};
use futures::future;
use tendermint::{account, chain};
use tokio::task::JoinHandle;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// URL type.
// TODO(tarcieri): use `url` crate?
pub type Url = String;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cosmoshub = {
        let chain_id = "cosmoshub-4".parse::<chain::Id>().unwrap();

        let validator_addr = "95E060D07713070FE9822F6C50BD76BCCBF9F17A"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = [
            "https://cosmos-rpc.polkachu.com/".into(),
            "https://cosmoshub.validator.network/".into(),
            //"https://rpc.cosmos.network:26657".into(),
        ];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    let osmosis = {
        let chain_id = "osmosis-1".parse::<chain::Id>().unwrap();

        let validator_addr = "20EFE186DA91A00AC7F042CD6CB6A1E882C583C7"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = [
            "https://osmosis-rpc.polkachu.com/".into(),
            "https://osmosis-rpc.publicnode.com/".into(),
            "https://rpc.dev-osmosis.zone/".into()
        ];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    future::join_all([cosmoshub, osmosis]).await;
}

async fn run_monitor(
    chain_id: chain::Id,
    validator_addr: account::Id,
    rpc_endpoints: impl IntoIterator<Item = Url> + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("[{chain_id}] monitoring signatures from {validator_addr}");

        let client_manager =
            ClientManager::new(rpc_endpoints).expect("couldn't initialize RPC client manager");

        let mut monitor = ChainMonitor::new(chain_id.clone(), client_manager).await;
        let missed_blocks_threshold = 5;

        loop {
            monitor.fetch_next_block().await;

            let missed_blocks = monitor.missed_blocks(validator_addr);

            if missed_blocks > missed_blocks_threshold {
                warn!("{} missed {} blocks!", &chain_id, missed_blocks);
            }
        }
    })
}
