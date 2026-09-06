use crate::{IBData, message::parse_message};

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
        _ => return Ok(()),
    }

    Ok(())
}
