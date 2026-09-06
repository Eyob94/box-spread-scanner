use std::{
    sync::{LazyLock, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    message::{Contract, IBKRMessageID, IBMessage},
};

static REQ_ID: LazyLock<Mutex<u32>> = LazyLock::new(|| Mutex::new(0));

pub fn request_spx_spot_price(tx: &UnboundedSender<Vec<u8>>) -> eyre::Result<()> {
    let contract = Contract {
        symbol: "SPX".into(),
        sec_type: "IND".into(),
        exchange: "CBOE".into(),
        trading_class: None,
        last_trade_date_or_contract_month: "".into(),
        ..Default::default()
    };

    let mut req_id = REQ_ID.lock().unwrap();
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMktData)
        .with_version(11)
        .field(req_id.to_string())
        .contract(contract)
        .field("0") // dnc
        .field("") // generic tick list
        .field("0") // snapshot
        .field("0") // regulatory snapshot
        .field(""); // mktDataOptions

    *req_id += 1;
    drop(req_id);

    tx.send(msg.into_bytes())?;

    Ok(())
}

pub fn request_delayed_market_data_type(tx: &UnboundedSender<Vec<u8>>) -> eyre::Result<()> {
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMarketDataType)
        .with_version(1)
        .field("3");

    tx.send(msg.into_bytes())?;

    Ok(())
}

pub fn request_spx_options_chain(tx: &UnboundedSender<Vec<u8>>) -> eyre::Result<()> {
    let mut req_id = REQ_ID.lock().unwrap();
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqSecDefOptParams)
        .field(req_id.to_string())
        .field("SPX")
        .field("")
        .field("IND")
        .field(416904.to_string());
    *req_id += 1;
    drop(req_id);
    tx.send(msg.into_bytes())?;
    Ok(())
}
