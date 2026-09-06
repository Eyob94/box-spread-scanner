use std::sync::{Arc, Mutex};

use axum::{Router, http::StatusCode, routing::get};
use tokio::{
    net::TcpListener,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tracing::instrument;

use crate::{Config, IBData, start_connection};

pub struct AppState {
    data: IBData,
    pub request_tx: UnboundedSender<Vec<u8>>,
    pub response_rx: UnboundedReceiver<Vec<u8>>,
}

#[instrument]
pub async fn start_server(config: Config) -> eyre::Result<()> {
    let (request_tx, request_rx) = unbounded_channel();
    let (response_tx, response_rx) = unbounded_channel();

    let data = start_connection(
        config.ib_port,
        config.client_id,
        response_tx.clone(),
        request_rx,
    )
    .await?;

    let app_state = AppState {
        request_tx,
        response_rx,
        data,
    };

    let router = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(Arc::new(Mutex::new(app_state)));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;

    axum::serve(listener, router).await?;
    Ok(())
}

