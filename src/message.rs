use serde::Serialize;
use strum_macros::Display;

#[derive(Debug, Clone, Default)]
pub struct IBMessage {
    id: IBKRMessageID,
    version: Option<u32>,
    fields: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Contract {
    pub con_id: Option<String>,
    pub symbol: String,
    pub sec_type: String,
    pub exchange: String,
    pub primary_exchange: Option<String>,
    pub currency: Currency,
    pub last_trade_date_or_contract_month: String,
    pub strike: Option<u32>,
    pub right: Option<OptionSide>,
    pub multiplier: Option<u32>,
    pub local_symbol: Option<String>,
    pub trading_class: Option<String>,
}

#[derive(Debug, Clone, Display, Eq, Hash, PartialEq, Default, Serialize)]
pub enum OptionSide {
    #[default]
    #[strum(serialize = "C")]
    Call,
    #[strum(serialize = "P")]
    Put,
}

#[derive(Debug, Clone, Display, Default)]
pub enum Currency {
    #[default]
    USD,
}

fn push_field(buf: &mut Vec<u8>, value: impl ToString) {
    buf.extend_from_slice(value.to_string().as_bytes());
    buf.push(0);
}

pub fn parse_message(byte_msg: &[u8]) -> eyre::Result<Vec<&str>> {
    byte_msg
        .split(|b| *b == 0)
        .filter(|x| !x.is_empty())
        .map(|a| std::str::from_utf8(a).map_err(Into::into))
        .collect::<eyre::Result<Vec<_>>>()
}

impl IBMessage {
    pub fn start_api_bytes(client_id: u16) -> Vec<u8> {
        let mut payload = vec![];
        push_field(&mut payload, IBKRMessageID::StartApi.into_wire_id());
        push_field(&mut payload, 2);
        push_field(&mut payload, client_id);
        push_field(&mut payload, "");
        payload
    }

    pub fn handshake() -> Vec<u8> {
        let version_range = b"v100..176";
        let mut msg = Vec::with_capacity(4 + 4 + version_range.len());
        msg.extend_from_slice(b"API\0");
        msg.extend_from_slice(&(version_range.len() as i32).to_be_bytes());
        msg.extend_from_slice(version_range);
        msg
    }

    pub fn with_id(mut self, id: IBKRMessageID) -> Self {
        self.id = id;
        self
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.fields.push(field.into());
        self
    }

    pub fn contract(mut self, contract: Contract) -> Self {
        let fields = &mut self.fields;
        fields.push(contract.con_id.unwrap_or_default());
        fields.push(contract.symbol);
        fields.push(contract.sec_type);
        fields.push(contract.last_trade_date_or_contract_month);
        fields.push(contract.strike.map(|s| s.to_string()).unwrap_or_default());
        fields.push(contract.right.map(|s| s.to_string()).unwrap_or_default());
        fields.push(
            contract
                .multiplier
                .map(|s| s.to_string())
                .unwrap_or_default(),
        );
        fields.push(contract.exchange);
        fields.push(contract.primary_exchange.unwrap_or_default());
        fields.push(contract.currency.to_string());
        fields.push(contract.local_symbol.unwrap_or_default());
        fields.push(contract.trading_class.unwrap_or_default());

        self
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut payload = vec![];
        push_field(&mut payload, self.id.into_wire_id());
        if let Some(version) = self.version {
            push_field(&mut payload, version);
        }
        for other in self.fields {
            push_field(&mut payload, other);
        }
        payload
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum IBKRMessageID {
    #[default]
    StartApi,
    ReqMktData,
    CancelMktData,
    ReqMarketDataType,
    ReqSecDefOptParams,
}

impl IBKRMessageID {
    pub fn into_wire_id(self) -> u32 {
        match self {
            Self::StartApi => 71,
            Self::ReqMktData => 1,

            Self::CancelMktData => 2,
            Self::ReqMarketDataType => 59,

            Self::ReqSecDefOptParams => 78,
        }
    }
}
