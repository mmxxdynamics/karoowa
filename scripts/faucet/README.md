# Karoowa Faucet

A simple Axum service that dispenses test tokens on the public devnet.

## Endpoint

```
POST /faucet
Content-Type: application/json

{"address": "0x..."}
```

Response: `{"tx_hash": "0x...", "amount": 1000000}`

## Rate Limiting

- 1 request per IP per 5 minutes
- Configurable via `FAUCET_RATE_LIMIT_SECS` env var

## Deployment

The faucet runs alongside the public devnet validator. It holds a treasury
key and signs transfers to requesting addresses.

## Status

**Not yet implemented** — ships as part of Phase 1.10 when the public
devnet VM is provisioned. The faucet will be a small standalone Rust binary
in this directory.

See `specs/development/dev_plan.md` T1.10.6 for the task spec.
