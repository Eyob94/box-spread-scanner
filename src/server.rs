use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    net::TcpListener,
    sync::mpsc::{UnboundedSender, unbounded_channel},
};
use tracing::{error, info, instrument};

use crate::{
    Config, IBData,
    actions::{
        request_delayed_market_data_type, request_spx_options_chain, request_spx_spot_price,
    },
    boxspread::{BoxSpread, evaluate_candidate},
    data::parse_ib_bytes,
    read_message_from_ibkr, send_message_to_ibkr, start_connection,
};

pub struct AppState {
    tx: UnboundedSender<Vec<u8>>,
    data: Arc<RwLock<IBData>>,
}

#[instrument]
pub async fn start_server(config: Config) -> eyre::Result<()> {
    let (request_tx, mut request_rx) = unbounded_channel();

    let (mut reader, mut writer, data) = start_connection(config.ib_port, config.client_id).await?;

    let data = Arc::new(RwLock::new(data));

    let app_state = AppState {
        data: data.clone(),
        tx: request_tx.clone(),
    };

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
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    let router = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/dates", get(get_available_dates))
        .route("/chains", get(get_spx_chain))
        .route("/boxes", post(get_boxes))
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
#[serde(rename_all = "camelCase")]
pub struct BoxBody {
    pub amount: u32,
    pub date: chrono::NaiveDate,
    pub exchange: String,
    pub trading_class: String,
}

#[allow(clippy::inconsistent_digit_grouping)]
const STEP_SIZES: [u32; 6] = [1000_00, 500_00, 250_00, 100_00, 50_00, 10_00]; // cents (100/50/20/10 points)

pub async fn get_boxes(
    State(app_state): State<Arc<AppState>>,
    Json(body): Json<BoxBody>,
) -> impl IntoResponse {
    let mut spread = BoxSpread::default()
        .with_loan(body.amount)
        .with_date(body.date);

    let (spot_price, chain) = {
        let data = app_state.data.read();
        let Some(spot_price) = data.spx_spot_price else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        let Some(chain) = data
            .spx_options_chains
            .get(&(body.exchange.clone(), body.trading_class.clone()))
        else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        (spot_price, chain.clone())
    };

    let [low_strike, high_strike] = spread.candidate_legs(spot_price, &chain.strikes);

    if low_strike == 0 || high_strike == 0 {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(e) = evaluate_candidate(
        &app_state.tx,
        &mut spread,
        body.date,
        body.exchange.clone(),
        body.trading_class.clone(),
        app_state.data.clone(),
    )
    .await
    {
        error!(?e, "Error evaluating candidate");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut best_spread = spread.clone();

    info!(liquidity = best_spread.liquidity, "Starting liquidity");

    let mut spreads = vec![];

    for step in STEP_SIZES {
        loop {
            let mut changed = false;
            let strikes = best_spread.strikes();

            let Some(low_strike) = strikes.iter().min() else {
                break;
            };
            let Some(high_strike) = strikes.iter().max() else {
                break;
            };
            info!(low_strike, high_strike, "Checking new strikes");
            spread.set_strikes(low_strike - step, high_strike - step);
            let down_liq = match evaluate_candidate(
                &app_state.tx,
                &mut spread,
                body.date,
                body.exchange.clone(),
                body.trading_class.clone(),
                app_state.data.clone(),
            )
            .await
            {
                Ok(d) => d,
                Err(e) => {
                    error!(?e, "Error evaluating");
                    break;
                }
            };

            spreads.push(spread.clone());

            info!(down_liq, "Down Liquidty");

            if down_liq > best_spread.liquidity {
                changed = true;
                best_spread = spread.clone();
            }
            spread.set_strikes(low_strike + step, high_strike + step);
            let up_liq = match evaluate_candidate(
                &app_state.tx,
                &mut spread,
                body.date,
                body.exchange.clone(),
                body.trading_class.clone(),
                app_state.data.clone(),
            )
            .await
            {
                Ok(u) => u,
                Err(e) => {
                    error!(?e, "Error evaluating");
                    break;
                }
            };
            spreads.push(spread.clone());

            info!(up_liq, "Up Liquidty");
            if up_liq > best_spread.liquidity {
                changed = true;
                best_spread = spread.clone();
            }

            info!(
                liquidity = best_spread.liquidity,
                step, changed, "Current best liquidity"
            );
            if !changed {
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    best_spread.calculate_box_pricing();
    for spread in spreads.iter_mut() {
        spread.calculate_box_pricing();
    }

    Json(json!({
        "best_spread": best_spread,
        "spreads": spreads
    }))
    .into_response()
}
