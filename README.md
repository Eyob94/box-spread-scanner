# Box Spread Scanner

This is a box spread scanner that checks multiple pairs from multiple exchanges - CBOE, IBSUPT, SMART on both SPX and SPXW, to find the best liquidity pair for a box spread

It's currently WIP

Usage

```bash
cargo run --bin chain

```

Will get the api server running.

Available environment variables

```bash
ib_port=[port of your ibkr connection, default is 4001 for IB gateway]
server_port=[api server port, default is 4045]
client_id=[client id to connect with id, default is 5]
```

### API

**To retrieve the SPX chain**

```bash
curl http://localhost:4045/chains
```

_Response_

```json

"IBUSOPT,SPXW": {
    "underlying_con_id": 416904,
    "multiplier": 100,
    "expirations": ["2026-09-08", "2026-09-09", "...", "2026-09-22"],
    "strikes": [640000, 660000, "...", 820000], // prices are in cents so 640000 is $6400.00
    "quotes": {}
},
"CBOE,SPX": {
    "underlying_con_id": 416904,
    "multiplier": 100,
    "expirations": ["2026-09-17", "2026-10-15", "...", "2027-02-18"],
    "strikes": [720000, 740000, "760000 // 7600.00 USD, results in cents", "...", 870000]
}

```

**Get expiration dates for each exchange and trading class combo**

```bash
curl http://localhost:4045/dates
```

_Response_

```json

"IBUSOPT,SPX": [
    ["2026-09-17", 11], // September 17, 2026 with 11 days to expiry from today
    ["2026-10-15", 39]
    // ...
],
"CBOE,SPXW": [
    ["2026-09-08", 2],
    ["2026-09-09", 3]
    // ...

```

**Find the best liquidity box spread for a given expiry**

```bash
curl -X POST http://localhost:4045/boxes \
  -d '{
        "tradingClass": "SPX",
        "exchange": "CBOE",
        "date": "2026-09-17",
        "amount": 10000000
      }'
```
Request body
```json
{
    "tradingClass": "SPX",   // SPX or SPXW
    "exchange": "CBOE",      // CBOE, IBSUPT, or SMART
    "date": "2026-09-17",    // expiration date, must be a valid date, check `/dates` if you need to see first
    "amount": 10000000       // amount in cents, so $100k becomes 100_000_00 (no dashes, simply for view)
}
```
_Response_

Starts from the nearest strikes to spot and walks outward in decreasing step sizes (looking for the box with the best liquidity, where liquidity is the min liquidity across its 4 legs). Returns the best spread found plus every candidate spread evaluated along the way.
```json
{
    "best_spread": {
        "date": "2026-09-17",
        "intended_loan": 10000000,
        "liquidity": 1212,           // liquidity score, the minimum one from 4 legs, becomes the limit
        "best_price": 99975,         // best price from the bid ask spread
        "mid_price": 99858,          
        "worst_price": 99740,        
        "best_rate_bps": 91,
        "mid_rate_bps": 519,
        "worst_rate_bps": 951,
        "legs": [
            {
                "option_side": "Call",
                "strike": 700000,     // 7000.00, cents
                "itm": true,
                "bid": 72320,
                "ask": 72420,
                "bid_size": 2,
                "ask_size": 2,
                "liquidity": 2856
            },
            // ...3 more legs (Call OTM, Put ITM, Put OTM)
        ]
    },
    "spreads": [
        // every candidate spread evaluated during the scan, similar structure to best spread
    ]
}
```
