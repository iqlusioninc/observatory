use std::{collections::VecDeque, time::Duration};
use tendermint::{account, block, chain, Block, Time};

/// Chain state tracker.
#[derive(Debug)]
pub struct ChainState {
    chain_id: chain::Id,
    blocks: VecDeque<BlockData>,
    history_size: usize,
}

impl ChainState {
    /// Number of blocks to retain.
    pub const DEFAULT_HISTORY_SIZE: usize = 100;

    /// Minimum expected consensus time.
    pub const MIN_CONSENSUS_TIME: Duration = Duration::from_secs(1);

    /// Create a new chain state.
    pub fn new(chain_id: chain::Id) -> Self {
        Self {
            chain_id,
            blocks: VecDeque::default(),
            history_size: Self::DEFAULT_HISTORY_SIZE,
        }
    }

    /// Get the chain ID.
    pub fn chain_id(&self) -> &chain::Id {
        &self.chain_id
    }

    /// Get the history size.
    pub fn history_size(&self) -> usize {
        self.history_size
    }

    /// Clear the current chain state, discarding all known blocks.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    /// Import a block into the chain state.
    pub fn import_block(&mut self, id: block::Id, block: Block) -> bool {
        let mut new_block = false;
        let block_data = BlockData { id, block };

        // TODO(tarcieri): make sure blocks are ordered in sequence
        if self.blocks.front().map(|entry| entry.id()) != Some(id) {
            self.blocks.push_front(block_data.clone());
            new_block = true;
        }

        if self.blocks.len() > self.history_size {
            self.blocks.resize(self.history_size, block_data)
        }

        new_block
    }

    /// Get the latest block if available.
    pub fn latest_block(&self) -> Option<&BlockData> {
        self.blocks.front()
    }

    /// Get estimated consensus time (i.e. average block production rate).
    pub fn consensus_time(&self) -> Duration {
        let mut consensus_times = Vec::with_capacity(self.blocks.len());

        if self.blocks.len() >= 2 {
            for i in 0..(self.blocks.len() - 1) {
                let a = &self.blocks[i];
                let b = &self.blocks[i + 1];

                if let Ok(duration) = a.time().duration_since(b.time()) {
                    consensus_times.push(duration);
                }
            }
        }

        if consensus_times.is_empty() {
            Self::MIN_CONSENSUS_TIME
        } else {
            consensus_times
                .iter()
                .sum::<Duration>()
                .div_f64(consensus_times.len() as f64)
        }
    }

    /// Get an estimated next block time.
    pub fn next_block_time(&self) -> Time {
        let last_time = self
            .latest_block()
            .map(|data| data.block.header.time)
            .unwrap_or(Time::now());

        (last_time + self.consensus_time()).unwrap_or(last_time)
    }

    /// Count the number of missed blocks for the given consensus key.
    pub fn missed_blocks(&self, validator_address: account::Id) -> usize {
        let mut result = 0;

        for data in &self.blocks {
            if let Some(commit) = &data.block.last_commit {
                let has_sig = commit.signatures.iter().any(|sig| {
                    sig.validator_address()
                        .map(|addr| addr == validator_address)
                        .unwrap_or(false)
                });

                if !has_sig {
                    result += 1;
                }
            }
        }

        result
    }
}

/// Data about a particular block in the chain.
#[derive(Clone, Debug)]
pub struct BlockData {
    id: block::Id,
    block: Block,
}

impl BlockData {
    /// Get the ID of this block.
    pub fn id(&self) -> block::Id {
        self.id
    }

    /// Get the block time.
    pub fn time(&self) -> Time {
        self.block.header.time
    }
}
