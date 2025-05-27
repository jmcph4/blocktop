use std::{
    net::SocketAddr,
    sync::Arc,
    thread::{self, JoinHandle},
};

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use log::error;
use prometheus::{Encoder, Registry, TextEncoder};
use tokio::net::TcpListener;
use tokio::runtime::Builder;

use crate::metrics::Metrics;

const NUM_WORKERS: usize = 1;

#[derive(Clone, Debug)]
pub struct MetricsService {
    pub metrics: Arc<Metrics>,
}

impl MetricsService {
    pub fn spawn(
        socket: SocketAddr,
        metrics: Arc<Metrics>,
    ) -> JoinHandle<eyre::Result<Self>> {
        thread::spawn(move || {
            let this = Self { metrics };
            let runtime = Builder::new_multi_thread()
                .worker_threads(NUM_WORKERS)
                .enable_all()
                .build()
                .inspect_err(|e| {
                    error!("Failed to initialise new Tokio runtime: {e:?}")
                })
                .unwrap();

            runtime.block_on(async move {
                let listener = TcpListener::bind(socket).await?;
                let registry_for_server = this.metrics.registry.clone();

                loop {
                    let (stream, _) = listener
                        .accept()
                        .await
                        .inspect_err(|e| {
                            error!(
                                "Failed to acquire TCP stream listener: {e:?}"
                            )
                        })
                        .unwrap();
                    let io = TokioIo::new(stream);
                    let registry_clone = Arc::clone(&registry_for_server);

                    tokio::task::spawn(async move {
                        let service = service_fn(move |req| {
                            serve_metrics(req, Arc::clone(&registry_clone))
                        });

                        http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                            .inspect_err(|e| error!("Failed to bind TCP connection for metrics: {e:?}"))
                            .unwrap();
                    });
                }
            })
        })
    }
}

async fn serve_metrics(
    req: Request<hyper::body::Incoming>,
    registry: Arc<Registry>,
) -> Result<Response<String>, std::convert::Infallible> {
    match req.uri().path() {
        "/metrics" => {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();

            match encoder.encode_to_string(&metric_families) {
                Ok(metrics_text) => Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", encoder.format_type())
                    .body(metrics_text)
                    .inspect_err(|e| {
                        error!("Failed to construct metrics response: {e:?}")
                    })
                    .unwrap()),
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Failed to encode metrics".to_string())
                    .inspect_err(|e| {
                        error!("Failed to construct metrics response: {e:?}")
                    })
                    .unwrap()),
            }
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not Found".to_string())
            .inspect_err(|e| {
                error!("Failed to construct metrics response: {e:?}")
            })
            .unwrap()),
    }
}
