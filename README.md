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
      "expirations": [
          "2026-09-08",
          "2026-09-09",
          "2026-09-10",
          "2026-09-11",
          "2026-09-14",
          "2026-09-15",
          "2026-09-16",
          "2026-09-17",
          "2026-09-18",
          "2026-09-21",
          "2026-09-22",
          ..
      ],
      "strikes": [
          ..
          640000,
          660000,
          680000,
          700000,
          720000,
          740000,
          760000,
          780000,
          800000,
          820000,
          ..
      ],
      "quotes": {}
  },
  "CBOE,SPX": {
      "underlying_con_id": 416904,
      "multiplier": 100,
      "expirations": [
          "2026-09-17",
          "2026-10-15",
          "2026-11-19",
          "2026-12-17",
          "2027-01-14",
          "2027-02-18",
          ..
      ],
      "strikes": [
          ..
          720000,
          740000,
          760000, // 7600.00 USD results are in cents
          780000,
          800000,
          820000,
          830000,
          840000,
          850000,
          860000,
          870000,
          ..
      ],
  ..
  }

```

**Get expiration dates for each exchange and trading class combo**

```bash
curl http://localhost:4045/dates
```

_Response_

```json

  "IBUSOPT,SPX": [
      [
          "2026-09-17",   // Sept 17
          11              // 11 days to expiry
      ],
      [
          "2026-10-15",
          39
      ],
      [
          "2026-11-19",
          74
      ],
      [
          "2026-12-17",
          102
      ],
      [
          "2027-01-14",
          130
      ],
  ],
  "CBOE,SPXW": [
      [
          "2026-09-08",
          2
      ],
      [
          "2026-09-09",
          3
      ],
      [
          "2026-09-10",
          4
      ],
      [
          "2026-09-11",
          5
      ],
      [
          "2026-09-14",
          8
      ],
      ..
  ],

```
