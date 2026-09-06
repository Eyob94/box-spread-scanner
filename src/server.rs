use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::mpsc::unbounded_channel};
use tracing::{info, instrument};

use crate::{
    Config, IBData,
    actions::{
        request_delayed_market_data_type, request_spx_options_chain, request_spx_spot_price,
    },
    boxspread::BoxSpread,
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
        request_spx_options_chain(&request_tx).unwrap();
        // check spx price every 5 seconds in case it updates
        loop {
            request_spx_spot_price(&request_tx).unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let router = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/dates", get(get_available_dates))
        .route("/chains", get(get_spx_chain))
        .with_state(Arc::new(app_state));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;

    axum::serve(listener, router).await?;
    Ok(())
}

pub async fn get_spx_chain(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = &app_state.data.read();

    info!(?data.spx_options_chains, "Chain");

    Json(
        data.spx_options_chains
            .iter()
            .map(|(k, v)| (format!("{},{}", k.0, k.1), v.clone()))
            .collect::<HashMap<String, _>>()
            .clone(),
    )
}

pub async fn get_available_dates(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = &app_state.data.read();

    let now = Utc::now().date_naive();

    Json(
        data.spx_options_chains
            .iter()
            .map(|(k, v)| {
                (
                    format!("{},{}", k.0, k.1),
                    v.expirations
                        .clone()
                        .into_iter()
                        .map(|ex| (ex, (ex - now).num_days()))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<HashMap<String, _>>()
            .clone(),
    )
}

#[derive(Serialize, Deserialize)]
pub struct BoxBody {
    pub loan_amount: u32,
    pub date: chrono::NaiveDate,
    pub exchange: String,
    pub trading_class: String,
}

pub async fn get_boxes(
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<BoxBody>,
) -> impl IntoResponse {
    let mut spread = BoxSpread::default()
        .with_loan(body.loan_amount)
        .with_date(body.date);

    let (spot_price, chain) = {
        let data = app_state.data.read();
        let Some(spot_price) = data.spx_spot_price else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        };

        let Some(chain) = data
            .spx_options_chains
            .get(&(body.exchange, body.trading_class))
        else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        };

        (spot_price, chain.clone())
    };

    let [low_strike, high_strike] = spread.candidate_legs(spot_price, &chain.strikes);

    if low_strike == 0 || high_strike == 0 {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}
