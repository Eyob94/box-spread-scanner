use std::{collections::HashMap};

use chrono::NaiveDate;
use eyre::OptionExt;

use crate::{IBData, OptionChainParams, message::parse_message};

pub fn parse_ib_bytes(payload: Vec<u8>, data: &mut IBData) -> eyre::Result<()> {
    let res = parse_message(&payload)?;

    if res.is_empty() {
        return Ok(());
    }

    match res[0] {
        "1" => {
            let (_, info) = res.split_at(2);
            let tick_type: i32 = info[1].parse()?;
            let price: f64 = info[2].parse()?;

            if matches!(tick_type, 75) && price > 0.0 {
                data.spx_spot_price = Some((price * 100.0).round() as u32);
            }
        }

        "75" => {
            let (_, info) = res.split_at(2);
            let exchange = info[0].to_string();
            let underlying_con_id: u32 = info[1].parse()?;

            let trading_class = info[2].to_string();
            let multiplier: u32 = info[3].parse()?;
            let num_expiries: usize = info[4].parse()?;

            let (expiry_strs, rest) = info[5..].split_at(num_expiries);

            let expirations = expiry_strs
                .iter()
                .map(|s| {
                    let year = &s[..4];
                    let month = &s[4..6];
                    let day = &s[6..8];
                    NaiveDate::from_ymd_opt(year.parse()?, month.parse()?, day.parse()?)
                        .ok_or_eyre("invalid expiry date")
                })
                .collect::<eyre::Result<Vec<NaiveDate>>>()?;

            let num_strikes: usize = rest[0].parse()?;
            let strike_strs = &rest[1..1 + num_strikes];
            let strikes = strike_strs
                .iter()
                .map(|s| -> eyre::Result<u32> {
                    let dollars: f64 = s.parse()?;
                    Ok((dollars * 100.0).round() as u32)
                })
                .collect::<eyre::Result<Vec<u32>>>()?;

            data.spx_options_chains.insert(
                (exchange, trading_class),
                OptionChainParams {
                    multiplier,
                    expirations,
                    strikes,
                    underlying_con_id,
                    quotes: HashMap::new(),
                },
            );
        }
        _ => return Ok(()),
    }

    Ok(())
}
