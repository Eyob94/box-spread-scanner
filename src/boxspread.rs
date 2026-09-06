use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{OptionQuote, message::OptionSide};

#[derive(Debug, Clone, Default, Serialize)]
pub struct BoxSpread {
    pub intended_loand: u32,
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
