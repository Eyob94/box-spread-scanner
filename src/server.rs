use std::{sync::Arc, time::Duration};

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use parking_lot::RwLock;
use tokio::{net::TcpListener, sync::mpsc::unbounded_channel};
use tracing::instrument;

use crate::{
    Config, IBData,
    actions::{request_delayed_market_data_type, request_spx_spot_price},
    data::parse_ib_bytes,
    read_message_from_ibkr, send_message_to_ibkr, start_connection,
};

pub struct AppState {
    data: Arc<RwLock<IBData>>,
}

#[instrument]
pub async fn start_server(config: Config) -> eyre::Result<()> {
    let (request_tx, mut request_rx) = unbounded_channel();

    let (mut reader, mut writer, data) = start_connection(config.ib_port, config.client_id).await?;

    let data = Arc::new(RwLock::new(data));

    let app_state = AppState { data: data.clone() };

    //

    tokio::spawn(async move {
        loop {
            let payload = read_message_from_ibkr(&mut reader).await.unwrap();

            parse_ib_bytes(payload, &mut data.write()).unwrap();
        }
    });

    tokio::spawn(async move {
        loop {
            let payload = request_rx.recv().await.unwrap();
            send_message_to_ibkr(&mut writer, payload).await.unwrap();
        }
    });

    tokio::spawn(async move {
        request_delayed_market_data_type(&request_tx).unwrap();
        // check spx price every 5 seconds in case it updates
        loop {
            request_spx_spot_price(&request_tx).unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let router = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/dates", get(get_available_dates))
        .with_state(Arc::new(app_state));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;

    axum::serve(listener, router).await?;
    Ok(())
}

pub async fn get_available_dates(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = &app_state.data;

    data.read().spx_spot_price.unwrap().to_string()
}
