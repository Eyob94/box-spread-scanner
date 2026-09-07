use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

use crate::message::{Contract, IBKRMessageID, IBMessage, OptionSide};

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
pub static PENDING_QUOTES: LazyLock<Mutex<HashMap<u32, (u32, OptionSide, String, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn request_option_quote(
    tx: &UnboundedSender<Vec<u8>>,
    strike: u32,
    right: OptionSide,
    expiry: &str,
    exchange: &str,
    trading_class: &str,
) -> eyre::Result<u32> {
    let contract = Contract {
        symbol: "SPX".into(),
        sec_type: "OPT".into(),
        exchange: exchange.into(),
        trading_class: Some(trading_class.into()),
        last_trade_date_or_contract_month: expiry.into(),
        strike: Some(strike),
        right: Some(right.clone()),
        multiplier: Some(100),
        ..Default::default()
    };

    let mut req_id = REQ_ID.lock().unwrap();
    let this_id = *req_id;
    *req_id += 1;
    drop(req_id);
    info!(?strike, ?right, "Sending quote for {strike}:{right}");

    PENDING_QUOTES.lock().unwrap().insert(
        this_id,
        (strike, right, exchange.into(), trading_class.into()),
    );

    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMktData)
        .with_version(11)
        .field(this_id.to_string())
        .contract(contract)
        .field("0")
        .field("")
        .field("0")
        .field("0")
        .field("");

    tx.send(msg.into_bytes())?;

    Ok(this_id)
}

pub fn cancel_market_data(tx: &UnboundedSender<Vec<u8>>, req_id: u32) -> eyre::Result<()> {
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::CancelMktData)
        .with_version(2)
        .field(req_id.to_string());

    tx.send(msg.into_bytes())?;

    Ok(())
}
