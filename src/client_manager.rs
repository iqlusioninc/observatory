use crate::Url;
use futures::future::{join_all, Future};
use std::{collections::BTreeMap as Map, time::Duration};
use tendermint_rpc::{error::Error as RpcError, HttpClient};
use tokio::time::timeout;
use tracing::warn;

/// Connection manager for RPC clients.
#[derive(Debug)]
pub struct ClientManager {
    /// Map of URLs to their corresponding RPC clients.
    clients: Map<Url, HttpClient>,

    /// Duration to use when making requests.
    timeout: Duration,
}

impl ClientManager {
    /// Default amount of time to wait for an RPC response.
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

    /// Create a new RPC client manager.
    pub fn new(urls: impl IntoIterator<Item = Url>) -> Result<Self, RpcError> {
        let mut clients = Map::new();

        for url in urls {
            let client = HttpClient::new(url.as_str())?;
            clients.insert(url, client);
        }

        Ok(Self {
            clients,
            timeout: Self::DEFAULT_TIMEOUT,
        })
    }

    /// Iterate over the RPC clients.
    pub fn clients(&self) -> impl Iterator<Item = &HttpClient> {
        self.clients.values()
    }

    /// Make a parallel request to all RPC clients.
    pub async fn request<'a, 'b, R, O, F>(&'a self, request: R) -> Vec<Result<O, RpcError>>
    where
        'a: 'b,
        R: Fn(&'b HttpClient) -> F,
        F: Future<Output = Result<O, RpcError>>,
    {
        let results = join_all(
            self.clients()
                .map(|client| timeout(self.timeout, request(client))),
        )
        .await;

        let mut responses = Vec::with_capacity(results.len());

        for (url, result) in self.clients.keys().zip(results) {
            match result {
                Ok(response) => responses.push(response),
                Err(e) => warn!("RPC timeout error for {}: {}", url, e),
            }
        }

        responses
    }
}
