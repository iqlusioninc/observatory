use crate::{chain_state::ChainState, client_manager::ClientManager};
use std::time::Duration;
use tendermint::{account, block, chain, Time};
use tendermint_rpc::{
    endpoint::block::Response as BlockResponse,
    error::{Error as RpcError, ErrorDetail as RpcErrorDetail},
    Client as _,
};
use tokio::time::sleep;
use tracing::{info, trace, warn};

/// Chain monitor which tracks current state.
#[derive(Debug)]
pub struct ChainMonitor {
    /// Chain state tracker.
    chain_state: ChainState,

    /// RPC clients used for monitoring.
    client_manager: ClientManager,

    /// Latest known block height.
    block_height: block::Height,

    /// Offset between the last wall time and the last block time
    bft_time_delta: Duration,
}

impl ChainMonitor {
    /// Create a new chain monitor from an RPC client manager.
    // TODO(tarcieri): error handling
    pub async fn new(chain_id: chain::Id, client_manager: ClientManager) -> Self {
        let mut chain_monitor = Self {
            chain_state: ChainState::new(chain_id),
            client_manager,
            block_height: block::Height::default(),
            bft_time_delta: Duration::ZERO,
        };

        let responses = chain_monitor
            .fetch_latest_blocks()
            .await
            .into_iter()
            .filter_map(|result| result.ok()) // TODO(tarcieri): log/handle errors
            .collect::<Vec<_>>();

        for response in &responses {
            let block_height = response.block.header.height;
            let chain_id = &response.block.header.chain_id;

            if chain_id != chain_monitor.chain_id() {
                // TODO(tarcieri): don't panic
                panic!(
                    "unexpected chain ID '{chain_id}'! (expecting {})",
                    chain_monitor.chain_id()
                );
            }

            if block_height > chain_monitor.block_height {
                chain_monitor.block_height = block_height;
            }
        }

        for response in responses {
            if response.block.header.height == chain_monitor.block_height {
                chain_monitor
                    .chain_state
                    .import_block(response.block_id, response.block);
            }
        }

        info!(
            "[{}] initialized at height {}",
            chain_monitor.chain_id(),
            block_height_with_commas(chain_monitor.block_height)
        );

        chain_monitor
    }

    /// Run the chain monitor.
    pub async fn fetch_next_block(&mut self) {
        if u64::from(self.block_height) % self.chain_state.history_size() as u64 == 0 {
            self.check_latest_blocks().await;
        }

        let started_at = Time::now();
        let next_height = self.block_height.increment();
        let next_block_time_without_offset = self.chain_state.next_block_time();
        let next_block_time = (next_block_time_without_offset + self.bft_time_delta)
            .unwrap_or(next_block_time_without_offset);

        loop {
            let sleep_duration = self.adjusted_sleep_duration(
                next_block_time
                    .duration_since(started_at)
                    .unwrap_or(ChainState::MIN_CONSENSUS_TIME),
            );

            trace!(
                "[{}] polling block {} in {:?}",
                self.chain_id(),
                next_height,
                sleep_duration
            );

            sleep(sleep_duration).await;

            let responses = self
                .client_manager
                .request(|client| client.block(next_height))
                .await;

            let mut added_block = false;

            for result in responses {
                match result {
                    Ok(response) => {
                        let now = Time::now();
                        let bft_time_delta = now
                            .duration_since(response.block.header.time)
                            .unwrap_or(Duration::ZERO);

                        let block_id = response.block_id;

                        if self.chain_state.import_block(block_id, response.block) {
                            added_block = true;
                            self.block_height = next_height;
                            self.bft_time_delta = bft_time_delta;

                            let duration = now.duration_since(started_at).unwrap_or(Duration::ZERO);

                            info!(
                                "[{}] imported block {} [{}] ({} secs)",
                                self.chain_id(),
                                block_height_with_commas(next_height),
                                &block_id.to_string()[..10],
                                duration.as_millis() as f64 / 1000.0
                            );
                        }
                    }
                    Err(err) => {
                        // RpcErrorDetail::Response is returned for unknown blocks, which are
                        // expected in the event that a new block hasn't yet been crated
                        if !matches!(err.detail(), RpcErrorDetail::Response(_)) {
                            warn!("[{}] RPC error: {}", self.chain_id(), err);
                        }
                    }
                }
            }

            if added_block {
                break;
            }
        }
    }

    /// Get the chain ID being monitored.
    pub fn chain_id(&self) -> &chain::Id {
        self.chain_state.chain_id()
    }

    /// Get the count of missed blocks for the given consensus key ID.
    pub fn missed_blocks(&self, validator_address: account::Id) -> usize {
        self.chain_state.missed_blocks(validator_address)
    }

    /// Get the count of recent blocks for the given consensus key ID.
    pub fn recent_blocks(&self, validator_address: account::Id) -> usize {
        self.chain_state.recent_blocks(validator_address)
    }

    /// Fetch the latest blocks for the given chain.
    async fn fetch_latest_blocks(&self) -> Vec<Result<BlockResponse, RpcError>> {
        self.client_manager
            .request(|client| client.latest_block())
            .await
            .into_iter()
            .collect()
    }

    /// Check if the state buffer is lagging too far behind the latest block height and if so, purge
    /// it and start over.
    async fn check_latest_blocks(&mut self) {
        let responses = self
            .fetch_latest_blocks()
            .await
            .into_iter()
            .filter_map(|result| result.ok()) // TODO(tarcieri): log/handle errors
            .collect::<Vec<_>>();

        let mut latest_block_height = self.block_height;

        for response in &responses {
            let block_height = response.block.header.height;

            if block_height > latest_block_height {
                latest_block_height = block_height;
            }
        }

        let delta = u64::from(latest_block_height) - u64::from(self.block_height);

        if delta > self.chain_state.history_size() as u64 {
            warn!(
                "[{}] monitor is {delta} blocks behind chain! Clearing history",
                self.chain_id()
            );

            self.chain_state.clear();
            self.block_height = latest_block_height;

            for response in responses {
                if response.block.header.height == latest_block_height {
                    self.chain_state
                        .import_block(response.block_id, response.block);
                }
            }
        }
    }

    /// Adjust a computed sleep duration based on our current computed offset between our local wall
    /// time and the chain's BFT time.
    fn adjusted_sleep_duration(&self, duration: Duration) -> Duration {
        // To avoid spurious requests we make the target offset slightly longer than the chain's
        // consensus time.
        let target_offset = self.chain_state.consensus_time().mul_f64(1.5);

        // Compute sleep time divisor based on how mich larger the actual time offset is versus our
        // idealized one based on the chain's consensus time.
        let mut divisor = self.bft_time_delta.as_millis() as f64 / target_offset.as_millis() as f64;

        // Make divisor increase exponentially with the offset.
        divisor *= divisor;

        // Ensure the divisor only decreases, not increases, the sleep duration.
        if divisor < 1.0 {
            divisor = 1.0;
        }

        duration.div_f64(divisor)
    }
}

/// Helper function to format block heights with commas
fn block_height_with_commas(height: block::Height) -> String {
    height
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .expect("block height should be a valid string")
        .join(",")
}
