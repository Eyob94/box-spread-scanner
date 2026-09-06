use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use eyre::{OptionExt, bail};
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    OptionQuote,
    actions::{request_delayed_market_data_type, request_option_quote},
    message::OptionSide,
};

#[derive(Debug, Clone, Default, Serialize)]
pub struct BoxSpread {
    pub intended_loan: u32,
    pub legs: [BoxSpreadLeg; 4],
    pub created: DateTime<Utc>,
    pub filled: Option<DateTime<Utc>>,
    pub date: NaiveDate,
    pub liquidity: u32,
    pub delta: Option<f64>,

    pub worst_price: u32,
    pub best_price: u32,
    pub mid_price: u32,
    pub worst_rate_bps: i64,

    pub mid_rate_bps: i64,
    pub best_rate_bps: i64,
}

impl BoxSpread {
    pub fn with_loan(self, loan: u32) -> Self {
        Self {
            intended_loan: loan,
            ..self
        }
    }

    pub fn with_date(self, date: chrono::NaiveDate) -> Self {
        Self { date, ..self }
    }

    pub fn strikes(&self) -> Vec<u32> {
        self.legs
            .iter()
            .filter(|l| l.option_side == OptionSide::Call)
            .map(|l| l.strike)
            .collect::<Vec<u32>>()
    }

    pub fn candidate_legs(&mut self, spot_price: u32, strikes: &[u32]) -> [u32; 2] {
        let width_cents = self.intended_loan;
        let width_points = width_cents / 10_000;
        let half_width_points = width_points / 2;

        let spot_points = spot_price / 100;
        let low_round =
            round_to_nearest(spot_points.saturating_sub(half_width_points), width_points);

        let high_round =
            round_to_nearest(spot_points.saturating_add(half_width_points), width_points);

        let low_strike = nearest_strike(strikes, low_round * 10_000);
        let high_strike = nearest_strike(strikes, high_round * 10_000);

        let (Some(low_strike), Some(high_strike)) = (low_strike, high_strike) else {
            return [0, 0];
        };

        self.legs = [
            BoxSpreadLeg {
                strike: low_strike,
                option_side: OptionSide::Call,
                itm: low_strike < spot_price,
                ..Default::default()
            },
            BoxSpreadLeg {
                strike: high_strike,
                option_side: OptionSide::Call,
                itm: high_strike < spot_price,
                ..Default::default()
            },
            BoxSpreadLeg {
                strike: high_strike,
                option_side: OptionSide::Put,
                itm: high_strike > spot_price,
                ..Default::default()
            },
            BoxSpreadLeg {
                strike: low_strike,
                option_side: OptionSide::Put,
                itm: low_strike > spot_price,
                ..Default::default()
            },
        ];
        [low_strike, high_strike]
    }
}

fn nearest_strike(strikes: &[u32], target: u32) -> Option<u32> {
    strikes
        .iter()
        .copied()
        .min_by_key(|&s| (s * 100).abs_diff(target))
}

fn round_to_nearest(value: u32, granularity: u32) -> u32 {
    if granularity == 0 {
        return value;
    }
    ((value + granularity / 2) / granularity) * granularity
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BoxSpreadLeg {
    pub strike: u32,
    pub option_side: OptionSide,
    pub itm: bool, // in the money?
    #[serde(flatten)]
    pub quote: Option<OptionQuote>,
    pub liquidity: u32,
    pub ideal: Option<u32>,
}

impl BoxSpreadLeg {
    fn calculate_liquidity_score(&mut self) -> u32 {
        let Some(quote) = &self.quote else {
            return 0;
        };

        let Some(bid) = quote.bid else {
            return 0;
        };

        let Some(ask) = quote.ask else {
            return 0;
        };
        if ask < bid {
            return 0;
        }

        let mid = (bid + ask) / 2;
        if mid == 0 {
            return 0;
        }

        let spread = ask - bid;
        let total_size = quote.bid_size.saturating_add(quote.ask_size);
        if total_size == 0 {
            return 0;
        }

        let relative_spread_bps = (spread as u64 * 10_000 / mid as u64) as u32;

        let spread_score = 10_000u32.saturating_div(relative_spread_bps.saturating_add(1));
        let size_score = total_size.min(10_000) as u32;

        self.liquidity = spread_score.saturating_mul(size_score);
        self.liquidity
    }
}

pub fn evaluate_candidate(
    tx: &UnboundedSender<Vec<u8>>,
    spread: &mut BoxSpread,
    expiration: NaiveDate,
    trading_class: &str,
) -> eyre::Result<u32> {
    let strikes = spread.strikes();

    let low_strike = strikes.iter().min().ok_or_eyre("No min strike")?;

    let high_strike = strikes.iter().max().ok_or_eyre("No max strike")?;

    let mut req_ids = vec![];

    request_delayed_market_data_type(&tx)?;

    for (strike, right) in [
        (low_strike, OptionSide::Call),
        (low_strike, OptionSide::Put),
        (high_strike, OptionSide::Call),
        (high_strike, OptionSide::Put),
    ] {
        let id = request_option_quote(
            tx,
            strike / 100,
            right,
            &expiration.to_string().replace("-", ""),
            trading_class,
        );
        req_ids.push(id);
    }

    loop {
        let start = std::time::Instant::now();
    }

    todo!()
}
