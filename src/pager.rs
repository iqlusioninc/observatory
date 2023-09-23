use std::{
    collections::BTreeMap as Map,
    fmt::{self, Debug},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tendermint::chain;
use tower::{Service, ServiceExt};
use tracing::warn;

/// Monitor the pager service for alarms, reporting them to the configured alerting service.
pub async fn monitor_pager_service(
    alerting_interval: Duration,
    mut service: tower::buffer::Buffer<PagerService, PagerRequest>,
) {
    loop {
        let response = service
            .ready()
            .await
            .expect("PagerService not ready")
            .call(PagerRequest::GetAlarms)
            .await
            .expect("PagerService error");

        let alarms = match response {
            PagerResponse::GetAlarms(alarms) => alarms,
            other => panic!("unexpected PagerService response: {:?}", other),
        };

        for alarm in alarms {
            report_alarm(alarm).await;
        }

        tokio::time::sleep(alerting_interval).await;
    }
}

/// Report a triggered alarm to the pager service.
async fn report_alarm(alarm: PagerAlarm) {
    warn!(
        "[{}] missed {} blocks!",
        alarm.chain_id, alarm.missed_blocks
    );
}

/// Pager service.
pub struct PagerService {
    /// Chain registry.
    chains: Map<chain::Id, usize>,

    /// Number of missing blocks after which an alert is created.
    missed_blocks_threshold: usize,

    /// Number of blocks after which we consider signing to be recovered.
    recovered_after_threshold: usize,
}

impl PagerService {
    pub fn new(missed_blocks_threshold: usize, recovered_after_threshold: usize) -> Self {
        Self {
            chains: Map::default(),
            missed_blocks_threshold,
            recovered_after_threshold,
        }
    }

    fn handle_event(&mut self, chain_id: chain::Id, missed_blocks: usize, recent_blocks: usize) {
        if recent_blocks >= self.recovered_after_threshold {
            self.chains.remove(&chain_id);
        } else if missed_blocks >= self.missed_blocks_threshold {
            self.chains.insert(chain_id, missed_blocks);
        }
    }

    fn get_alarms(&mut self) -> Vec<PagerAlarm> {
        let result = self
            .chains
            .iter()
            .map(|(chain_id, missed_blocks)| PagerAlarm {
                chain_id: chain_id.clone(),
                missed_blocks: *missed_blocks,
            })
            .collect();

        self.chains.clear();
        result
    }
}

impl Service<PagerRequest> for PagerService {
    type Response = PagerResponse;
    type Error = PagerError;
    type Future = Pin<Box<dyn Future<Output = Result<PagerResponse, PagerError>> + Send + 'static>>;

    fn poll_ready(&mut self, _ctx: &mut Context<'_>) -> Poll<Result<(), PagerError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: PagerRequest) -> Self::Future {
        let response = match request {
            PagerRequest::Event {
                chain_id,
                missed_blocks,
                recent_blocks,
            } => {
                self.handle_event(chain_id, missed_blocks, recent_blocks);
                Ok(PagerResponse::Event)
            }
            PagerRequest::GetAlarms => Ok(PagerResponse::GetAlarms(self.get_alarms())),
        };
        Box::pin(async { response })
    }
}

/// Pager alarms which indicate something is wrong and a page should be sent.
#[derive(Debug)]
pub struct PagerAlarm {
    /// Chain ID the alarm is for.
    pub chain_id: chain::Id,

    /// Number of missed blocks.
    // TODO(tarcieri): other types of alarms?
    pub missed_blocks: usize,
}

/// Requests sent to the pager service.
#[derive(Debug)]
pub enum PagerRequest {
    /// Report information to the pager.
    Event {
        /// Chain ID where event occurred.
        chain_id: chain::Id,

        /// Number of blocks that have been missed in the past 100.
        missed_blocks: usize,

        /// Number of blocks since the last miss which have been signed.
        recent_blocks: usize,
    },

    /// Get alarms for the pager.
    GetAlarms,
}

/// Response sent from the pager service.
#[derive(Debug)]
pub enum PagerResponse {
    /// Event responses contain no data.
    Event,

    /// Get alarams response with the alarms.
    GetAlarms(Vec<PagerAlarm>),
}

/// Error type.
#[derive(Debug)]
pub struct PagerError;

impl fmt::Display for PagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("pager error")
    }
}

impl std::error::Error for PagerError {}
