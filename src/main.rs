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

    let agoric = {
        let chain_id = "agoric-3".parse::<chain::Id>().unwrap();

        let validator_addr = "D1CE9A9EF19196DA9BCEA8484791DC6BA28178B0"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = [
            "https://agoric-rpc.polkachu.com/".into(),
            "https://main.rpc.agoric.net/".into(),
        ];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    let cosmoshub = {
        let chain_id = "cosmoshub-4".parse::<chain::Id>().unwrap();

        let validator_addr = "95E060D07713070FE9822F6C50BD76BCCBF9F17A"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = [
            "https://cosmos-rpc.polkachu.com/".into(),
            "https://cosmoshub.validator.network/".into(),
        ];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    let neutron = {
        let chain_id = "neutron-1".parse::<chain::Id>().unwrap();

        let validator_addr = "0161BE816E9B2D368D1717D21650C216DF3F627C"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = ["https://neutron-rpc.polkachu.com/".into()];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    let noble = {
        let chain_id = "noble-1".parse::<chain::Id>().unwrap();

        let validator_addr = "9814A41D7ADECFC8686C1B551CFE12A5529CCF47"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = ["https://noble-rpc.polkachu.com/".into()];

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
            "https://rpc.dev-osmosis.zone/".into(),
        ];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    let stride = {
        let chain_id = "stride-1".parse::<chain::Id>().unwrap();

        let validator_addr = "D542FA46ABFB3D29FE3E284D4380DE231A4791C8"
            .parse::<account::Id>()
            .unwrap();

        let rpc_urls = ["https://stride-rpc.polkachu.com/".into()];

        run_monitor(chain_id, validator_addr, rpc_urls).await
    };

    future::join_all([agoric, cosmoshub, neutron, noble, osmosis, stride]).await;
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
        let missed_blocks_threshold = 3;

        loop {
            monitor.fetch_next_block().await;

            let missed_blocks = monitor.missed_blocks(validator_addr);

            if missed_blocks > missed_blocks_threshold {
                warn!("{} missed {} blocks!", &chain_id, missed_blocks);
            }
        }
    })
}
