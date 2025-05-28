use std::sync::Arc;

use prometheus::{IntGauge, Opts, Registry};

#[derive(Clone, Debug)]
pub struct Metrics {
    pub rpc_requests: Arc<IntGauge>,
    pub blocks_added: Arc<IntGauge>,
    pub failed_rpc_requests: Arc<IntGauge>,
    pub registry: Arc<Registry>,
}

impl Metrics {
    pub fn new() -> Self {
        let rpc_requests = IntGauge::with_opts(Opts::new(
            "rpc_requests",
            "The number of requests made to the RPC node",
        ))
        .expect("Invalid rpc_requests gauge definition");
        let blocks_added = IntGauge::with_opts(Opts::new(
            "blocks_added",
            "The number of blocks added to the index",
        ))
        .expect("Invalid blocks_added gauge definition");
        let failed_rpc_requests = IntGauge::with_opts(Opts::new(
            "failed_rpc_requests",
            "The number of requests made to the RPC node that have received an error response",
        ))
        .expect("Invalid rpc_requests gauge definition");
        let registry = Registry::new();
        registry
            .register(Box::new(rpc_requests.clone()))
            .expect("Invalid metrics registry definition");
        registry
            .register(Box::new(blocks_added.clone()))
            .expect("Invalid metrics registry definition");
        registry
            .register(Box::new(failed_rpc_requests.clone()))
            .expect("Invalid metrics registry definition");

        Self {
            rpc_requests: Arc::new(rpc_requests),
            blocks_added: Arc::new(blocks_added),
            failed_rpc_requests: Arc::new(failed_rpc_requests),
            registry: Arc::new(registry),
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
